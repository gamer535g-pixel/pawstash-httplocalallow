use crate::config::settings::AppSettings;
use reqwest::{Client, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Serialize)]
pub struct CreateAccountRequest<'a> {
    pub account_id: &'a str,
    pub auth_token: &'a str,
    pub kdf: &'a serde_json::Value,
    pub encrypted_key_bundle: &'a str,
    pub nonce: &'a str,
}
#[derive(Debug, Deserialize)]
pub struct BundleResponse {
    #[serde(default)]
    pub account_id: Option<String>,
    pub kdf: serde_json::Value,
    pub encrypted_key_bundle: String,
    pub nonce: String,
}
#[derive(Serialize)]
pub struct ReplaceBundleRequest<'a> {
    pub kdf: &'a serde_json::Value,
    pub encrypted_key_bundle: &'a str,
    pub nonce: &'a str,
}
#[derive(Debug, Deserialize)]
pub struct SessionResponse {
    pub token: String,
    pub expires_at: String,
    pub protocol_version: u32,
}
#[derive(Serialize)]
struct SessionRequest<'a> {
    account_id: &'a str,
    auth_token: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_id: Option<&'a str>,
}
#[derive(Serialize)]
struct DeviceRequest<'a> {
    id: &'a str,
    name: &'a str,
    platform: &'a str,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncDevice {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}
#[derive(Debug, Deserialize)]
struct DevicesResponse {
    devices: Vec<SyncDevice>,
}
#[derive(Debug, Deserialize)]
pub struct PullResponse {
    pub cursor: i64,
    pub changes: Vec<RemoteChange>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteChange {
    pub position: i64,
    pub record_id: String,
    pub revision: i64,
    pub kind: String,
    pub device_id: String,
    pub ciphertext: String,
    pub nonce: String,
    pub tombstone: bool,
    pub created_at: String,
}
#[derive(Serialize)]
struct PushRequest<'a> {
    protocol_version: u32,
    records: Vec<PushRecord<'a>>,
}
#[derive(Serialize)]
struct PushRecord<'a> {
    record_id: &'a str,
    kind: &'a str,
    expected_revision: i64,
    device_id: &'a str,
    ciphertext: &'a str,
    nonce: &'a str,
    tombstone: bool,
}
#[derive(Clone, Debug)]
pub struct PushRecordInput<'a> {
    pub record_id: &'a str,
    pub kind: &'a str,
    pub expected_revision: i64,
    pub device_id: &'a str,
    pub ciphertext: &'a str,
    pub nonce: &'a str,
    pub tombstone: bool,
}
#[derive(Debug, Deserialize)]
pub struct PushResponse {
    pub cursor: i64,
    pub accepted: Vec<AcceptedRecord>,
}
#[derive(Debug, Deserialize)]
pub struct AcceptedRecord {
    pub record_id: String,
    pub revision: i64,
    pub position: i64,
}

pub const PULL_PAGE_LIMIT: usize = 500;

/// Plain HTTP is accepted only for loopback and private-network hosts
/// (localhost, 127.0.0.0/8, 10/8, 172.16/12, 192.168/16, 169.254/16,
/// 100.64/10 used by Tailscale, and IPv6 ::1 / fc00::/7).
fn is_plain_http_allowed(host: &str) -> bool {
    use std::net::IpAddr;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => {
            let o = ip.octets();
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || (o[0] == 100 && (o[1] & 0xC0) == 64)
        }
        Ok(IpAddr::V6(ip)) => ip.is_loopback() || (ip.segments()[0] & 0xfe00) == 0xfc00,
        Err(_) => false,
    }
}

pub struct SyncHttpClient {
    client: Client,
    base: String,
}
impl SyncHttpClient {
    pub fn new(server_url: &str, settings: &AppSettings) -> Result<Self, String> {
        let url = Url::parse(server_url.trim()).map_err(|e| e.to_string())?;
        let local = url.host_str().is_some_and(is_plain_http_allowed);
        if url.scheme() != "https" && !local {
            return Err("Sync server requires HTTPS (plain HTTP is only allowed for localhost and private-network addresses)".to_string());
        }
        let builder = crate::net::builder_with_proxy(settings)?
            .timeout(crate::net::defaults().request_timeout);
        let base = format!("{}/v1", server_url.trim().trim_end_matches('/'));
        Ok(Self {
            client: builder.build().map_err(|e| e.to_string())?,
            base,
        })
    }
    pub async fn create_account(&self, value: &CreateAccountRequest<'_>) -> Result<(), String> {
        self.send_empty(
            self.client
                .post(format!("{}/accounts", self.base))
                .json(value),
            StatusCode::CREATED,
        )
        .await
    }
    pub async fn bundle(&self, account: &str) -> Result<BundleResponse, String> {
        self.send_json(
            self.client.get(format!(
                "{}/accounts/{}/bundle",
                self.base,
                urlencoding::encode(account)
            )),
            StatusCode::OK,
        )
        .await
    }
    pub async fn replace_bundle(
        &self,
        token: &str,
        account: &str,
        value: &ReplaceBundleRequest<'_>,
    ) -> Result<(), String> {
        let response = self
            .client
            .put(format!(
                "{}/accounts/{}/bundle",
                self.base,
                urlencoding::encode(account)
            ))
            .bearer_auth(token)
            .json(value)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Sync replace_bundle request failed: {e}");
                e.to_string()
            })?;
        if response.status() != StatusCode::OK && response.status() != StatusCode::NO_CONTENT {
            let err = Self::error(response).await;
            tracing::error!("Sync replace_bundle error: {err}");
            return Err(err);
        }
        Ok(())
    }
    pub async fn session(
        &self,
        account: &str,
        auth_token: &str,
        device_id: Option<&str>,
    ) -> Result<SessionResponse, String> {
        self.send_json(
            self.client
                .post(format!("{}/sessions", self.base))
                .json(&SessionRequest {
                    account_id: account,
                    auth_token,
                    device_id,
                }),
            StatusCode::CREATED,
        )
        .await
    }
    pub async fn register_device(&self, token: &str, id: &str, name: &str) -> Result<(), String> {
        self.send_empty(
            self.client
                .post(format!("{}/devices", self.base))
                .bearer_auth(token)
                .json(&DeviceRequest {
                    id,
                    name,
                    platform: std::env::consts::OS,
                }),
            StatusCode::CREATED,
        )
        .await
    }
    pub async fn list_devices(&self, token: &str) -> Result<Vec<SyncDevice>, String> {
        self.send_json::<DevicesResponse>(
            self.client
                .get(format!("{}/devices", self.base))
                .bearer_auth(token),
            StatusCode::OK,
        )
        .await
        .map(|response| response.devices)
    }
    pub async fn revoke_device(&self, token: &str, device_id: &str) -> Result<(), String> {
        self.send_empty(
            self.client
                .delete(format!(
                    "{}/devices/{}",
                    self.base,
                    urlencoding::encode(device_id)
                ))
                .bearer_auth(token),
            StatusCode::NO_CONTENT,
        )
        .await
    }
    pub async fn pull(&self, token: &str, after: i64) -> Result<PullResponse, String> {
        self.send_json(
            self.client
                .get(format!("{}/changes", self.base))
                .bearer_auth(token)
                .query(&[("after", after), ("limit", PULL_PAGE_LIMIT as i64)]),
            StatusCode::OK,
        )
        .await
    }
    pub async fn push(
        &self,
        token: &str,
        idempotency: &str,
        records: &[PushRecordInput<'_>],
    ) -> Result<PushResponse, String> {
        let body = PushRequest {
            protocol_version: 1,
            records: records
                .iter()
                .map(|record| PushRecord {
                    record_id: record.record_id,
                    kind: record.kind,
                    expected_revision: record.expected_revision,
                    device_id: record.device_id,
                    ciphertext: record.ciphertext,
                    nonce: record.nonce,
                    tombstone: record.tombstone,
                })
                .collect(),
        };
        self.send_json(
            self.client
                .post(format!("{}/records:push", self.base))
                .bearer_auth(token)
                .header("Idempotency-Key", idempotency)
                .json(&body),
            StatusCode::OK,
        )
        .await
    }

    pub async fn ping(&self) -> Result<(), String> {
        let url = format!("{}/health", self.base);
        tracing::debug!("Pinging sync server at {url}");
        let response = self.client.get(&url).send().await.map_err(|e| {
            tracing::warn!("Sync server health ping failed: {e}");
            format!("Connection failed: {e}")
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!("Sync server health ping returned HTTP {status}: {body}");
            return Err(format!("Server returned HTTP {status}: {}", body.trim()));
        }
        tracing::info!("Sync server health ping succeeded: HTTP 200");
        Ok(())
    }

    async fn send_empty(
        &self,
        builder: reqwest::RequestBuilder,
        expected: StatusCode,
    ) -> Result<(), String> {
        let response = builder.send().await.map_err(|e| {
            tracing::error!("Sync HTTP request failed: {e}");
            e.to_string()
        })?;
        if response.status() != expected {
            let err = Self::error(response).await;
            tracing::error!("Sync HTTP error: {err}");
            return Err(err);
        }
        Ok(())
    }
    async fn send_json<T: DeserializeOwned>(
        &self,
        builder: reqwest::RequestBuilder,
        expected: StatusCode,
    ) -> Result<T, String> {
        let response = builder.send().await.map_err(|e| {
            let details = crate::net::error_chain(&e);
            tracing::error!("Sync HTTP request network error: {details}");
            format!("Network error: {details}")
        })?;
        if response.status() != expected {
            let err = Self::error(response).await;
            tracing::error!("Sync HTTP error: {err}");
            return Err(err);
        }
        let text = response.text().await.map_err(|e| {
            tracing::error!("Sync HTTP read body error: {e}");
            format!("Failed to read response body: {e}")
        })?;
        serde_json::from_str::<T>(&text).map_err(|e| {
            tracing::error!("Sync HTTP JSON deserialize error: {e}, payload: {text}");
            format!("Failed to decode server response ({e}): {text}")
        })
    }
    async fn error(response: reqwest::Response) -> String {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        format!("Sync server HTTP {status}: {}", body.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::crypto::{
        decrypt_record, encrypt_record, unwrap_vault, wrap_vault, KdfEnvelope, VaultSecrets,
    };
    use uuid::Uuid;

    fn unique(prefix: &str) -> String {
        format!("{prefix}_{}", Uuid::new_v4().simple())
    }

    fn fast_kdf() -> KdfEnvelope {
        KdfEnvelope {
            memory_kib: 1024,
            iterations: 1,
            ..KdfEnvelope::generate()
        }
    }

    async fn device_session(
        client: &SyncHttpClient,
        account: &str,
        auth_token: &str,
        name: &str,
    ) -> (String, String) {
        let bootstrap = client.session(account, auth_token, None).await.unwrap();
        let device = unique("device");
        client
            .register_device(&bootstrap.token, &device, name)
            .await
            .unwrap();
        let session = client
            .session(account, auth_token, Some(&device))
            .await
            .unwrap();
        (device, session.token)
    }

    #[tokio::test]
    #[ignore = "requires a local pawstash sync-server in PAWSTASH_TEST_SYNC_URL"]
    async fn two_devices_exchange_only_encrypted_records() {
        let server = std::env::var("PAWSTASH_TEST_SYNC_URL").unwrap();
        let client = SyncHttpClient::new(&server, &AppSettings::default()).unwrap();
        let account = unique("account");
        let password = "integration master password";
        let kdf = fast_kdf();
        let secrets = VaultSecrets::generate();
        let wrapped = wrap_vault(password, &kdf, &secrets).unwrap();
        client
            .create_account(&CreateAccountRequest {
                account_id: &account,
                auth_token: &secrets.auth_token,
                kdf: &serde_json::to_value(&kdf).unwrap(),
                encrypted_key_bundle: &wrapped.ciphertext,
                nonce: &wrapped.nonce,
            })
            .await
            .unwrap();
        let (first_device, first_token) =
            device_session(&client, &account, &secrets.auth_token, "Device A").await;

        let record_id = unique("record");
        let plaintext = br#"{"schema_version":1,"collections":[],"entries":[],"memberships":[],"subscriptions":[]}"#;
        let encrypted =
            encrypt_record(&secrets.vault_key_bytes().unwrap(), &record_id, plaintext).unwrap();
        client
            .push(
                &first_token,
                &unique("request"),
                &[PushRecordInput {
                    record_id: &record_id,
                    kind: "collection",
                    expected_revision: 0,
                    device_id: &first_device,
                    ciphertext: &encrypted.ciphertext,
                    nonce: &encrypted.nonce,
                    tombstone: false,
                }],
            )
            .await
            .unwrap();

        let bundle = client.bundle(&account).await.unwrap();
        let second_secrets = unwrap_vault(
            password,
            &serde_json::from_value(bundle.kdf).unwrap(),
            &bundle.encrypted_key_bundle,
            &bundle.nonce,
        )
        .unwrap();
        let (_, second_token) =
            device_session(&client, &account, &second_secrets.auth_token, "Device B").await;
        let pulled = client.pull(&second_token, 0).await.unwrap();
        assert_eq!(pulled.changes.len(), 1);
        let change = &pulled.changes[0];
        let opened = decrypt_record(
            &second_secrets.vault_key_bytes().unwrap(),
            &change.record_id,
            &change.ciphertext,
            &change.nonce,
        )
        .unwrap();
        assert_eq!(opened, plaintext);

        let devices = client.list_devices(&second_token).await.unwrap();
        assert_eq!(devices.iter().filter(|d| d.revoked_at.is_none()).count(), 2);

        let new_password = "replacement integration password";
        let new_kdf = fast_kdf();
        let replacement = wrap_vault(new_password, &new_kdf, &second_secrets).unwrap();
        client
            .replace_bundle(
                &second_token,
                &account,
                &ReplaceBundleRequest {
                    kdf: &serde_json::to_value(&new_kdf).unwrap(),
                    encrypted_key_bundle: &replacement.ciphertext,
                    nonce: &replacement.nonce,
                },
            )
            .await
            .unwrap();
        let replaced = client.bundle(&account).await.unwrap();
        let replaced_kdf: KdfEnvelope = serde_json::from_value(replaced.kdf).unwrap();
        let open_with = |pass| {
            unwrap_vault(
                pass,
                &replaced_kdf,
                &replaced.encrypted_key_bundle,
                &replaced.nonce,
            )
        };
        assert!(open_with(password).is_err());
        assert!(open_with(new_password).is_ok());

        client
            .revoke_device(&second_token, &first_device)
            .await
            .unwrap();
        assert!(client
            .session(&account, &secrets.auth_token, Some(&first_device))
            .await
            .is_err());
    }
}
