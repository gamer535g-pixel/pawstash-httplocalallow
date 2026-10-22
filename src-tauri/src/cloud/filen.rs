use super::models::{CloudFolderResult, CloudNode};
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures_util::stream::{self, StreamExt};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha512};

const GATEWAY: &str = "https://gateway.filen.io";
const EGEST: &str = "https://egest.filen.io";
pub const CHUNK_SIZE: u64 = 1024 * 1024;
const CHUNK_OVERHEAD: usize = 12 + 16;
const MAX_FOLDERS: usize = 2000;
const LISTING_CONCURRENCY: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilenLink {
    pub uuid: String,
    pub key: String,
}

#[derive(Deserialize)]
struct Envelope<T> {
    status: bool,
    #[serde(default)]
    message: String,
    data: Option<T>,
}

#[derive(Deserialize)]
struct LinkInfo {
    parent: String,
    metadata: String,
    #[serde(rename = "hasPassword")]
    has_password: bool,
}

#[derive(Deserialize)]
struct LinkContent {
    folders: Vec<LinkFolder>,
    files: Vec<LinkFile>,
}

#[derive(Deserialize)]
struct LinkFolder {
    uuid: String,
    metadata: String,
}

#[derive(Deserialize)]
struct LinkFile {
    uuid: String,
    metadata: String,
    size: u64,
    bucket: String,
    region: String,
    version: u32,
}

#[derive(Deserialize)]
struct FolderMetadata {
    name: String,
}

#[derive(Deserialize)]
struct FileMetadata {
    name: String,
    key: String,
    #[serde(default)]
    mime: Option<String>,
}

pub fn supports_url(url: &str) -> bool {
    parse_filen_url(url).is_some()
}

pub fn parse_filen_url(url: &str) -> Option<FilenLink> {
    if !super::url_host_matches(url, &["filen.io"]) {
        return None;
    }
    let parsed = reqwest::Url::parse(url.trim()).ok()?;
    let fragment = urlencoding::decode(parsed.fragment()?).ok()?;
    let (uuid, key) = fragment.strip_prefix("/f/")?.split_once('#')?;
    let is_uuid = uuid.len() == 36 && uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    let key = decode_hex_key(key).unwrap_or_else(|| key.to_string());
    (is_uuid && !key.is_empty()).then(|| FilenLink {
        uuid: uuid.to_ascii_lowercase(),
        key,
    })
}

fn decode_hex_key(raw: &str) -> Option<String> {
    if raw.len() != 64 || !raw.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let bytes = (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    String::from_utf8(bytes)
        .ok()
        .filter(|key| key.chars().all(|c| c.is_ascii_graphic()))
}

fn decrypt_metadata<T: DeserializeOwned>(metadata: &str, key: &str) -> Result<T, String> {
    let rest = metadata
        .strip_prefix("002")
        .ok_or("Unsupported Filen metadata format")?;
    if rest.len() <= 12 || !rest.is_char_boundary(12) {
        return Err("Malformed Filen metadata".into());
    }
    let (iv, data) = rest.split_at(12);
    let mut derived = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha512>(key.as_bytes(), key.as_bytes(), 1, &mut derived);
    let ciphertext = STANDARD
        .decode(data)
        .map_err(|e| format!("Malformed Filen metadata: {e}"))?;
    let plain = Aes256Gcm::new_from_slice(&derived)
        .map_err(|e| e.to_string())?
        .decrypt(Nonce::from_slice(iv.as_bytes()), ciphertext.as_ref())
        .map_err(|_| "Could not decrypt the Filen link. The key in it may be wrong".to_string())?;
    serde_json::from_slice(&plain).map_err(|e| format!("Unexpected Filen metadata: {e}"))
}

async fn gateway<T: DeserializeOwned>(
    client: &Client,
    path: &str,
    body: serde_json::Value,
) -> Result<T, String> {
    let resp = client
        .post(format!("{GATEWAY}{path}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Filen request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Filen returned HTTP {}", resp.status()));
    }
    let envelope: Envelope<T> = resp
        .json()
        .await
        .map_err(|e| format!("Unexpected Filen response: {e}"))?;
    match (envelope.status, envelope.data) {
        (true, Some(data)) => Ok(data),
        _ => Err(format!("Filen: {}", envelope.message)),
    }
}

pub async fn resolve_filen(client: &Client, url: &str) -> Result<CloudFolderResult, String> {
    let link = parse_filen_url(url).ok_or("Invalid Filen link")?;
    let info: LinkInfo = gateway(client, "/v3/dir/link/info", json!({ "uuid": link.uuid })).await?;
    if info.has_password {
        return Err("Password-protected Filen links are not supported yet".into());
    }
    let title = decrypt_metadata::<FolderMetadata>(&info.metadata, &link.key)?.name;
    let password = format!("{:x}", Sha512::digest(b"empty"));

    let mut nodes = vec![folder_node(info.parent.clone(), None, title.clone())];
    let mut level = vec![info.parent];
    let mut folder_count = 1;
    while !level.is_empty() {
        let listings: Vec<(String, Result<LinkContent, String>)> = stream::iter(level)
            .map(|parent| {
                let body = json!({ "uuid": link.uuid, "parent": parent, "password": password });
                async move {
                    let content = gateway(client, "/v3/dir/link/content", body).await;
                    (parent, content)
                }
            })
            .buffer_unordered(LISTING_CONCURRENCY)
            .collect()
            .await;

        let mut next = Vec::new();
        for (parent, content) in listings {
            let content = content?;
            for folder in content.folders {
                let name = decrypt_metadata::<FolderMetadata>(&folder.metadata, &link.key)?.name;
                folder_count += 1;
                if folder_count > MAX_FOLDERS {
                    return Err(format!("Filen folder has more than {MAX_FOLDERS} folders"));
                }
                nodes.push(folder_node(folder.uuid.clone(), Some(parent.clone()), name));
                next.push(folder.uuid);
            }
            for file in content.files {
                let meta = decrypt_metadata::<FileMetadata>(&file.metadata, &link.key)?;
                nodes.push(file_node(&file, meta, &parent));
            }
        }
        level = next;
    }

    let files = nodes.iter().filter(|node| !node.is_folder);
    Ok(CloudFolderResult {
        provider: "filen".into(),
        url: url.trim().to_string(),
        title,
        total_files: files.clone().count() as u64,
        total_size: files.filter_map(|node| node.size).sum(),
        is_single_file: false,
        nodes,
    })
}

fn folder_node(id: String, parent_id: Option<String>, name: String) -> CloudNode {
    CloudNode {
        id,
        parent_id,
        name,
        size: None,
        is_folder: true,
        mime_type: None,
        download_url: None,
        stream_url: None,
        thumbnail_url: None,
        children: None,
        is_encrypted: false,
    }
}

fn file_node(file: &LinkFile, meta: FileMetadata, parent: &str) -> CloudNode {
    let stream_url = (file.version == 2 && meta.key.len() == 32).then(|| {
        format!(
            "/cloud_stream/filen?region={}&bucket={}&uuid={}&key={}&size={}&name={}",
            urlencoding::encode(&file.region),
            urlencoding::encode(&file.bucket),
            urlencoding::encode(&file.uuid),
            urlencoding::encode(&meta.key),
            file.size,
            urlencoding::encode(&meta.name)
        )
    });
    let mime_type = meta.mime.filter(|mime| !mime.is_empty()).or_else(|| {
        mime_guess::from_path(&meta.name)
            .first_raw()
            .map(str::to_string)
    });
    CloudNode {
        id: file.uuid.clone(),
        parent_id: Some(parent.to_string()),
        name: meta.name,
        size: Some(file.size),
        is_folder: false,
        mime_type,
        download_url: stream_url.clone(),
        stream_url,
        thumbnail_url: None,
        children: None,
        is_encrypted: false,
    }
}

pub fn chunk_url(region: &str, bucket: &str, uuid: &str, index: u64) -> String {
    format!(
        "{EGEST}/{}/{}/{}/{index}",
        urlencoding::encode(region),
        urlencoding::encode(bucket),
        urlencoding::encode(uuid)
    )
}

pub fn chunk_cipher(key: &str) -> Result<Aes256Gcm, String> {
    Aes256Gcm::new_from_slice(key.as_bytes()).map_err(|_| "Invalid Filen file key".to_string())
}

pub fn decrypt_chunk(cipher: &Aes256Gcm, data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < CHUNK_OVERHEAD {
        return Err("Truncated Filen chunk".into());
    }
    cipher
        .decrypt(Nonce::from_slice(&data[..12]), &data[12..])
        .map_err(|_| "Could not decrypt a Filen chunk".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINK: &str = "https://app.filen.io/#/f/ed204269-428e-42e0-af64-220471a7a124%236576743441697556695335625a584f446c7967716842427567575f6334453839";

    #[test]
    fn parses_hex_encoded_and_plain_keys() {
        let expected = FilenLink {
            uuid: "ed204269-428e-42e0-af64-220471a7a124".into(),
            key: "evt4AiuViS5bZXODlygqhBBugW_c4E89".into(),
        };
        assert_eq!(parse_filen_url(LINK), Some(expected.clone()));
        assert_eq!(
            parse_filen_url(
                "https://app.filen.io/#/f/ed204269-428e-42e0-af64-220471a7a124#evt4AiuViS5bZXODlygqhBBugW_c4E89"
            ),
            Some(expected)
        );
        assert_eq!(
            parse_filen_url("https://app.filen.io/#/d/ed204269-428e-42e0-af64-220471a7a124%23abc"),
            None
        );
        assert_eq!(
            parse_filen_url("https://example.com/#/f/ed204269-428e-42e0-af64-220471a7a124%23abc"),
            None
        );
    }

    #[test]
    fn decrypts_link_metadata() {
        let meta: FolderMetadata = decrypt_metadata(
            "002dBePgLfXKex-eNSwgVYEVs2rjKhyU1Jj72jDY1FMiyPnTm2SVXTtWEU=",
            "evt4AiuViS5bZXODlygqhBBugW_c4E89",
        )
        .unwrap();
        assert_eq!(meta.name, "Doggy");
        assert!(decrypt_metadata::<FolderMetadata>(
            "002dBePgLfXKex-eNSwgVYEVs2rjKhyU1Jj72jDY1FMiyPnTm2SVXTtWEU=",
            "wrongwrongwrongwrongwrongwrong12",
        )
        .is_err());
    }

    #[test]
    fn chunk_round_trip() {
        let cipher = chunk_cipher("yPYIWAmhagLWuldeNWb21LmFvH8b_1VF").unwrap();
        let iv = [7u8; 12];
        let mut sealed = iv.to_vec();
        sealed.extend(
            cipher
                .encrypt(Nonce::from_slice(&iv), b"ftypmp42".as_ref())
                .unwrap(),
        );
        assert_eq!(decrypt_chunk(&cipher, &sealed).unwrap(), b"ftypmp42");
        assert!(decrypt_chunk(&cipher, &sealed[..20]).is_err());
    }
}
