mod extract;
mod password;
mod reader;

pub use password::extract_password_candidates;

use crate::cloud::{CloudFolderResult, CloudNode};
use crate::db::archives::ArchivePasswordRepository;
use crate::db::content::ContentRepository;
use crate::server::axum_server::UpstreamGateway;
use axum::body::Body;
use axum::http::StatusCode;
use axum::response::Response;
use extract::{entry_response, stored_entry_range, Extraction};
use reader::{HostPacer, LocalReader, RemoteReader, RemoteSource, SourceReader, Transport};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Read, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::OnceCell;
use zip::read::{ArchiveOffset, Config, ZipArchiveMetadata, ZipFileEntry, ZipReadOptions};
use zip::result::ZipError;
use zip::{CompressionMethod, HasZipMetadata, ZipArchive};

const ORIGIN_DOCUMENT: &str = "archive_source";
const MAX_SESSIONS: usize = 8;
const ZIPCRYPTO_CHECKS: usize = 3;
const EXTRACT_BUFFER: usize = 256 * 1024;
const LOCAL_HEADER_SLACK: u64 = 128 * 1024;
const PASSTHROUGH_CHUNK: usize = 256 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct ArchiveOpenRequest {
    pub url: Option<String>,
    pub local_path: Option<String>,
    pub name: String,
    pub service: String,
    pub creator_id: String,
    pub post_id: String,
    pub post_text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveOpenResult {
    pub archive_id: String,
    pub format: String,
    pub status: String,
    pub encrypted: bool,
    pub password_source: Option<String>,
    pub password_hints: Vec<String>,
    pub folder: Option<CloudFolderResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchiveOrigin {
    source: String,
    local: bool,
    name: String,
    service: String,
    creator_id: String,
    post_id: String,
}

#[derive(Debug, Clone, PartialEq)]
struct EntryInfo {
    path: String,
    is_dir: bool,
    size: u64,
    compressed_size: u64,
    header_start: u64,
    stored: bool,
    encrypted: bool,
    aes: bool,
}

#[derive(Clone)]
struct ActivePassword {
    value: String,
    source: &'static str,
}

enum SourceHandle {
    Remote(Arc<RemoteSource>),
    Local {
        path: PathBuf,
        file: Arc<std::fs::File>,
        len: u64,
    },
}

impl SourceHandle {
    fn local(path: &Path) -> Result<Self, String> {
        let open = || -> std::io::Result<_> {
            let file = std::fs::File::open(path)?;
            let len = file.metadata()?.len();
            Ok((file, len))
        };
        let (file, len) =
            open().map_err(|e| format!("Could not open '{}': {e}", path.display()))?;
        Ok(Self::Local {
            path: path.to_path_buf(),
            file: Arc::new(file),
            len,
        })
    }

    fn len(&self) -> u64 {
        match self {
            Self::Remote(remote) => remote.len(),
            Self::Local { len, .. } => *len,
        }
    }

    fn reader(&self, sequential: Option<Range<u64>>) -> SourceReader {
        match self {
            Self::Remote(remote) => {
                SourceReader::Remote(RemoteReader::new(remote.clone(), sequential))
            }
            Self::Local { file, len, .. } => {
                SourceReader::Local(LocalReader::new(file.clone(), *len))
            }
        }
    }
}

enum SessionError {
    Unknown,
    Failed(String),
}

impl From<String> for SessionError {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

impl SessionError {
    fn message(self) -> String {
        match self {
            Self::Unknown => {
                "This archive is no longer open; open it again from the post".to_string()
            }
            Self::Failed(message) => message,
        }
    }
}

struct ArchiveSession {
    id: String,
    origin: ArchiveOrigin,
    source: SourceHandle,
    metadata: Arc<ZipArchiveMetadata>,
    entries: Vec<EntryInfo>,
    password: RwLock<Option<ActivePassword>>,
    extractions: Mutex<HashMap<usize, Arc<Extraction>>>,
}

fn normalize_entry_path(name: &str) -> String {
    name.replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .collect::<Vec<_>>()
        .join("/")
}

fn entry_info(entry: &ZipFileEntry<'_>) -> Result<EntryInfo, String> {
    let name = entry
        .name()
        .map_err(|e| format!("Unreadable entry name in archive: {e}"))?;
    Ok(EntryInfo {
        path: normalize_entry_path(&name),
        is_dir: entry.is_dir(),
        size: entry.size(),
        compressed_size: entry.compressed_size(),
        header_start: entry.header_start(),
        stored: entry.compression() == CompressionMethod::Stored,
        encrypted: entry.encrypted(),
        aes: entry.get_metadata().aes_settings().is_some(),
    })
}

fn describe_zip_error(path: &str, error: ZipError) -> String {
    match error {
        ZipError::InvalidPassword => format!("Wrong password for '{path}'"),
        ZipError::UnsupportedArchive(reason) => format!("'{path}' can't be opened here: {reason}"),
        other => format!("Could not read '{path}' from the archive: {other}"),
    }
}

impl ArchiveSession {
    fn load(id: String, origin: ArchiveOrigin, source: SourceHandle) -> Result<Self, String> {
        // Offset detection scans the whole file for headers, so remote archives must not use it.
        let at_start = Config {
            archive_offset: ArchiveOffset::Known(0),
        };
        let archive = match ZipArchive::with_config(at_start, source.reader(None)) {
            Ok(archive) => archive,
            Err(_) if matches!(source, SourceHandle::Local { .. }) => {
                ZipArchive::new(source.reader(None))
                    .map_err(|e| format!("Could not read the archive's file list: {e}"))?
            }
            Err(error) => {
                return Err(format!(
                    "Could not read the archive's file list ({error}); if it is a self-extracting archive, download it first"
                ))
            }
        };
        let metadata = archive.metadata();
        let entries = (0..metadata.len())
            .map(|index| {
                metadata
                    .entry(index)
                    .map_err(|e| format!("Could not read archive entry {index}: {e}"))
                    .and_then(|entry| entry_info(&entry))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            id,
            origin,
            source,
            metadata,
            entries,
            password: RwLock::new(None),
            extractions: Mutex::new(HashMap::new()),
        })
    }

    fn is_encrypted(&self) -> bool {
        self.entries.iter().any(|entry| entry.encrypted)
    }

    fn password(&self) -> Option<ActivePassword> {
        self.password
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn set_password(&self, value: String, source: &'static str) {
        *self
            .password
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(ActivePassword { value, source });
    }

    fn entry(&self, index: usize) -> Result<ZipFileEntry<'_>, String> {
        self.metadata
            .entry(index)
            .map_err(|e| format!("Archive entry {index} is missing: {e}"))
    }

    fn verify_password(&self, password: &str) -> Result<bool, String> {
        let mut encrypted: Vec<usize> = (0..self.entries.len())
            .filter(|index| self.entries[*index].encrypted && !self.entries[*index].is_dir)
            .collect();
        encrypted.sort_by_key(|index| self.entries[*index].compressed_size);
        let checks = match encrypted.first() {
            Some(index) if self.entries[*index].aes => 1,
            Some(_) => ZIPCRYPTO_CHECKS,
            None => return Ok(true),
        };
        for index in encrypted.into_iter().take(checks) {
            let mut reader = self.source.reader(None);
            let options = ZipReadOptions::new().password(Some(password.as_bytes()));
            let opened = self.entry(index)?.with_reader(&mut reader, options);
            match opened {
                Ok(_) => {}
                Err(ZipError::InvalidPassword) => return Ok(false),
                Err(error) => return Err(describe_zip_error(&self.entries[index].path, error)),
            }
        }
        Ok(true)
    }

    fn data_start(&self, index: usize) -> Result<u64, String> {
        let mut reader = self.source.reader(None);
        self.entry(index)?
            .get_metadata()
            .data_start(&mut reader)
            .map_err(|e| describe_zip_error(&self.entries[index].path, e))
    }

    fn extract_entry(
        &self,
        index: usize,
        out: &mut impl Write,
        mut progress: impl FnMut(u64) -> Result<(), String>,
    ) -> Result<u64, String> {
        let info = self
            .entries
            .get(index)
            .ok_or_else(|| format!("Archive entry {index} is missing"))?;
        let sequential = match self.source {
            SourceHandle::Remote(_) => Some(
                info.header_start
                    ..info
                        .header_start
                        .saturating_add(info.compressed_size)
                        .saturating_add(LOCAL_HEADER_SLACK)
                        .min(self.source.len()),
            ),
            SourceHandle::Local { .. } => None,
        };
        let mut reader = self.source.reader(sequential);
        let password = self.password().map(|active| active.value);
        let options = ZipReadOptions::new().password(password.as_deref().map(str::as_bytes));
        let mut file = self
            .entry(index)?
            .with_reader(&mut reader, options)
            .map_err(|e| describe_zip_error(&info.path, e))?;
        let mut buffer = vec![0u8; EXTRACT_BUFFER];
        let mut written = 0u64;
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|e| format!("Could not extract '{}': {e}", info.path))?;
            if count == 0 {
                break;
            }
            out.write_all(&buffer[..count])
                .map_err(|e| format!("Could not write '{}' to the cache: {e}", info.path))?;
            written += count as u64;
            if written > info.size {
                return Err(format!(
                    "'{}' is larger than the archive says ({} bytes)",
                    info.path, info.size
                ));
            }
            progress(written)?;
        }
        if written != info.size {
            return Err(format!(
                "'{}' extracted to {written} bytes instead of {}",
                info.path, info.size
            ));
        }
        Ok(written)
    }

    fn extraction(self: &Arc<Self>, index: usize) -> Result<Arc<Extraction>, String> {
        let info = self
            .entries
            .get(index)
            .ok_or_else(|| format!("Archive entry {index} is missing"))?;
        let mut extractions = self
            .extractions
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(existing) = extractions.get(&index).filter(|job| job.is_reusable()) {
            return Ok(existing.clone());
        }
        let extension = Path::new(&info.path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(crate::db::storage::sanitize_cache_key)
            .filter(|ext| !ext.is_empty() && ext.len() <= 16)
            .unwrap_or_else(|| "bin".to_string());
        let directory = extraction_cache_dir().join(&self.id);
        std::fs::create_dir_all(&directory)
            .map_err(|e| format!("Could not create the archive cache folder: {e}"))?;
        let job = Extraction::new(directory.join(format!("{index}.{extension}")), info.size);
        extractions.insert(index, job.clone());
        drop(extractions);

        let session = self.clone();
        let worker = job.clone();
        tokio::task::spawn_blocking(move || session.run_extraction(index, &worker));
        Ok(job)
    }

    fn run_extraction(&self, index: usize, job: &Extraction) {
        let result = std::fs::File::create(job.path())
            .map_err(|e| format!("Could not create the archive cache file: {e}"))
            .and_then(|mut file| {
                self.extract_entry(index, &mut file, |written| job.progress(written))?;
                file.flush()
                    .map_err(|e| format!("Could not write the archive cache file: {e}"))
            });
        if let Err(message) = &result {
            if let Err(error) = std::fs::remove_file(job.path()) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(path = %job.path().display(), %error, "Could not remove a partial archive extraction");
                }
            }
            if job.is_cancelled() {
                tracing::debug!(archive = %self.id, index, %message, "Archive extraction stopped");
            } else {
                tracing::error!(archive = %self.id, index, %message, "Archive extraction failed");
            }
        }
        job.finish(result);
    }

    fn has_running_extraction(&self) -> bool {
        self.extractions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .any(|job| job.is_running())
    }

    fn cancel_extractions(&self) {
        for job in self
            .extractions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
        {
            job.cancel();
        }
    }

    fn result(&self, password_hints: Vec<String>) -> ArchiveOpenResult {
        let encrypted = self.is_encrypted();
        let active = self.password();
        ArchiveOpenResult {
            archive_id: self.id.clone(),
            format: "zip".to_string(),
            status: if encrypted && active.is_none() {
                "password_required"
            } else {
                "ready"
            }
            .to_string(),
            encrypted,
            password_source: active.map(|active| active.source.to_string()),
            password_hints,
            folder: Some(CloudFolderResult {
                provider: "archive".to_string(),
                url: self.origin.source.clone(),
                title: self.origin.name.clone(),
                total_files: self.entries.iter().filter(|entry| !entry.is_dir).count() as u64,
                total_size: self
                    .entries
                    .iter()
                    .filter(|entry| !entry.is_dir)
                    .map(|entry| entry.size)
                    .sum(),
                is_single_file: false,
                nodes: build_nodes(&self.id, &self.entries),
            }),
        }
    }
}

fn build_nodes(archive_id: &str, entries: &[EntryInfo]) -> Vec<CloudNode> {
    let mut nodes = Vec::new();
    let mut folders = HashSet::new();
    for (index, entry) in entries.iter().enumerate() {
        let parts: Vec<&str> = entry
            .path
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();
        let Some(file_name) = parts.last() else {
            continue;
        };
        let folder_depth = if entry.is_dir {
            parts.len()
        } else {
            parts.len() - 1
        };
        let mut parent: Option<String> = None;
        for depth in 1..=folder_depth {
            let folder_path = parts[..depth].join("/");
            let id = format!("{archive_id}:d:{folder_path}");
            if folders.insert(folder_path) {
                nodes.push(CloudNode {
                    id: id.clone(),
                    parent_id: parent.clone(),
                    name: parts[depth - 1].to_string(),
                    size: None,
                    is_folder: true,
                    mime_type: None,
                    download_url: None,
                    stream_url: None,
                    thumbnail_url: None,
                    children: None,
                    is_encrypted: false,
                });
            }
            parent = Some(id);
        }
        if entry.is_dir {
            continue;
        }
        let url = format!("/cloud_stream/archive?id={archive_id}&entry={index}");
        nodes.push(CloudNode {
            id: format!("{archive_id}:f:{index}"),
            parent_id: parent,
            name: file_name.to_string(),
            size: Some(entry.size),
            is_folder: false,
            mime_type: mime_guess::from_path(file_name)
                .first()
                .map(|mime| mime.essence_str().to_string()),
            download_url: Some(url.clone()),
            stream_url: Some(url),
            thumbnail_url: None,
            children: None,
            is_encrypted: entry.encrypted,
        });
    }
    nodes
}

fn detect_format(name: &str) -> &'static str {
    let lower = name.trim().to_lowercase();
    let without_part = lower
        .rsplit_once('.')
        .filter(|(_, suffix)| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
        .map_or(lower.as_str(), |(stem, _)| stem);
    if lower.ends_with(".zip") || lower.ends_with(".cbz") {
        "zip"
    } else if lower.ends_with(".rar") || lower.ends_with(".cbr") {
        "rar"
    } else if without_part.ends_with(".7z") || lower.ends_with(".cb7") {
        "7z"
    } else {
        "unknown"
    }
}

fn archive_id_for(source: &str) -> String {
    format!("{:x}", Sha256::digest(source.as_bytes()))
}

fn extraction_cache_dir() -> PathBuf {
    crate::db::storage::content_cache_path().join("archives")
}

pub fn clear_extraction_cache() -> Result<(), String> {
    match std::fs::remove_dir_all(extraction_cache_dir()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "Could not clear the archive extraction cache: {error}"
        )),
    }
}

#[derive(Default)]
struct Sessions {
    cells: HashMap<String, Arc<OnceCell<Arc<ArchiveSession>>>>,
    order: VecDeque<String>,
}

const LOCAL_STREAM_PREFIX: &str = "/cloud_stream/";
const ARCHIVE_ROUTE: &str = "/cloud_stream/archive";

fn is_own_stream_route(url: &reqwest::Url) -> bool {
    url.scheme() == "http"
        && matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
        && url.path().starts_with(LOCAL_STREAM_PREFIX)
}

// Decrypted Mega files reach us through our own media server; store them without the per-launch token.
fn local_stream_source(raw: &str) -> Option<Result<String, String>> {
    let url = reqwest::Url::parse(raw)
        .or_else(|_| reqwest::Url::parse("http://127.0.0.1/")?.join(raw))
        .ok()?;
    if !is_own_stream_route(&url) {
        return None;
    }
    if url.path().starts_with(ARCHIVE_ROUTE) {
        return Some(Err(
            "Archives inside other archives can't be opened yet".to_string()
        ));
    }
    let query: Vec<String> = url
        .query_pairs()
        .filter(|(name, _)| name != "token")
        .map(|(name, value)| {
            format!(
                "{}={}",
                urlencoding::encode(&name),
                urlencoding::encode(&value)
            )
        })
        .collect();
    Some(Ok(if query.is_empty() {
        url.path().to_string()
    } else {
        format!("{}?{}", url.path(), query.join("&"))
    }))
}

pub struct ArchiveRegistry {
    upstream: Arc<UpstreamGateway>,
    loopback: reqwest::Client,
    media_port: Arc<std::sync::atomic::AtomicU16>,
    media_token: String,
    content: Arc<ContentRepository>,
    passwords: Arc<ArchivePasswordRepository>,
    pacer: Arc<HostPacer>,
    sessions: Mutex<Sessions>,
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| format!("Archive worker failed: {e}"))?
}

impl ArchiveRegistry {
    pub fn new(
        upstream: Arc<UpstreamGateway>,
        content: Arc<ContentRepository>,
        passwords: Arc<ArchivePasswordRepository>,
        media_port: Arc<std::sync::atomic::AtomicU16>,
        media_token: String,
    ) -> Result<Self, String> {
        let loopback = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("Failed to build the local media client: {e}"))?;
        Ok(Self {
            upstream,
            loopback,
            media_port,
            media_token,
            content,
            passwords,
            pacer: Arc::new(HostPacer::new(reader::HOST_REQUEST_INTERVAL)),
            sessions: Mutex::new(Sessions::default()),
        })
    }

    pub fn passwords(&self) -> &ArchivePasswordRepository {
        &self.passwords
    }

    pub fn clear(&self) {
        let mut sessions = self.sessions.lock().unwrap_or_else(PoisonError::into_inner);
        for cell in sessions.cells.values() {
            if let Some(session) = cell.get() {
                session.cancel_extractions();
            }
        }
        *sessions = Sessions::default();
    }

    fn session_cell(&self, id: &str) -> Arc<OnceCell<Arc<ArchiveSession>>> {
        let mut sessions = self.sessions.lock().unwrap_or_else(PoisonError::into_inner);
        sessions.order.retain(|existing| existing != id);
        sessions.order.push_back(id.to_string());
        if let Some(cell) = sessions.cells.get(id) {
            return cell.clone();
        }
        let cell = Arc::new(OnceCell::new());
        sessions.cells.insert(id.to_string(), cell.clone());
        while sessions.cells.len() > MAX_SESSIONS {
            let idle = sessions.order.iter().position(|candidate| {
                candidate != id
                    && sessions.cells.get(candidate).is_some_and(|cell| {
                        cell.get()
                            .is_some_and(|session| !session.has_running_extraction())
                    })
            });
            let Some(position) = idle else {
                break;
            };
            if let Some(evicted) = sessions.order.remove(position) {
                sessions.cells.remove(&evicted);
            }
        }
        cell
    }

    async fn connect(&self, origin: &ArchiveOrigin) -> Result<SourceHandle, String> {
        if origin.local {
            let path = PathBuf::from(&origin.source);
            return blocking(move || SourceHandle::local(&path)).await;
        }
        let (transport, raw) = if origin.source.starts_with(LOCAL_STREAM_PREFIX) {
            let port = self.media_port.load(std::sync::atomic::Ordering::Acquire);
            let absolute = crate::server::absolute_local_media_url(
                origin.source.clone(),
                port,
                &self.media_token,
            )?;
            (Transport::Loopback(self.loopback.clone()), absolute)
        } else {
            (
                Transport::Upstream(self.upstream.clone()),
                origin.source.clone(),
            )
        };
        let url = reqwest::Url::parse(&raw).map_err(|e| format!("Invalid archive URL: {e}"))?;
        let remote = RemoteSource::open(transport, self.pacer.clone(), url).await?;
        Ok(SourceHandle::Remote(remote))
    }

    fn persisted_origin(&self, id: &str) -> Result<Option<ArchiveOrigin>, String> {
        if let Some(origin) =
            self.content
                .load_document::<ArchiveOrigin>(ORIGIN_DOCUMENT, id, "", "")?
        {
            return Ok(Some(origin));
        }
        let Some(source) = self.passwords.source_for_archive(id)? else {
            return Ok(None);
        };
        let local = !(source.starts_with("https://")
            || source.starts_with("http://")
            || source.starts_with(LOCAL_STREAM_PREFIX));
        let name = source
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
            .to_string();
        Ok(Some(ArchiveOrigin {
            source,
            local,
            name,
            service: String::new(),
            creator_id: String::new(),
            post_id: String::new(),
        }))
    }

    async fn session(
        &self,
        id: &str,
        origin: Option<ArchiveOrigin>,
    ) -> Result<Arc<ArchiveSession>, SessionError> {
        let cell = self.session_cell(id);
        let session = cell
            .get_or_try_init(|| async {
                let origin = match origin {
                    Some(origin) => origin,
                    None => self.persisted_origin(id)?.ok_or(SessionError::Unknown)?,
                };
                if archive_id_for(&origin.source) != id {
                    return Err(SessionError::Unknown);
                }
                let source = self.connect(&origin).await?;
                let session_id = id.to_string();
                let loaded_origin = origin.clone();
                let session =
                    blocking(move || ArchiveSession::load(session_id, loaded_origin, source)).await?;
                if let Err(error) = self.content.save_document(ORIGIN_DOCUMENT, id, "", "", &origin)
                {
                    tracing::warn!(archive = %id, %error, "Could not persist the archive source; queued downloads will not survive a restart");
                }
                Ok(Arc::new(session))
            })
            .await;
        match session {
            Ok(session) => Ok(session.clone()),
            Err(error) => {
                let mut sessions = self.sessions.lock().unwrap_or_else(PoisonError::into_inner);
                if sessions.cells.get(id).is_some_and(|existing| {
                    Arc::ptr_eq(existing, &cell) && existing.get().is_none()
                }) {
                    sessions.cells.remove(id);
                    sessions.order.retain(|existing| existing != id);
                }
                Err(error)
            }
        }
    }

    async fn verify(&self, session: &Arc<ArchiveSession>, password: &str) -> Result<bool, String> {
        let session = session.clone();
        let password = password.to_string();
        blocking(move || session.verify_password(&password)).await
    }

    fn remember(&self, session: &ArchiveSession, service: &str, creator_id: &str, password: &str) {
        if let Err(error) = self.passwords.remember(
            &session.id,
            service,
            creator_id,
            password,
            &session.origin.source,
        ) {
            tracing::warn!(archive = %session.id, %error, "Could not remember the archive password");
        }
    }

    async fn origin_for(&self, request: &ArchiveOpenRequest) -> Result<ArchiveOrigin, String> {
        let base = |source: String, local: bool| ArchiveOrigin {
            source,
            local,
            name: request.name.trim().to_string(),
            service: request.service.clone(),
            creator_id: request.creator_id.clone(),
            post_id: request.post_id.clone(),
        };
        if let Some(path) = request
            .local_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            let path = PathBuf::from(path);
            let canonical = blocking(move || {
                Ok(dunce::canonicalize(&path)
                    .ok()
                    .filter(|resolved| resolved.is_file()))
            })
            .await?;
            if let Some(resolved) = canonical {
                return Ok(base(resolved.to_string_lossy().into_owned(), true));
            }
        }
        let url = request
            .url
            .as_deref()
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .ok_or_else(|| "This archive has no download URL and no downloaded copy".to_string())?;
        if let Some(local_stream) = local_stream_source(url) {
            return Ok(base(local_stream?, false));
        }
        let parsed = reqwest::Url::parse(&crate::downloader::normalize_download_url(url))
            .map_err(|e| format!("Invalid archive URL: {e}"))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("Archive URLs must use HTTP or HTTPS".to_string());
        }
        Ok(base(parsed.to_string(), false))
    }

    pub fn can_list(name: &str) -> bool {
        detect_format(name) == "zip"
    }

    pub async fn list_entry_names(&self, url: &str, name: &str) -> Result<Vec<String>, String> {
        if !Self::can_list(name) {
            return Err(format!("'{name}' is not a ZIP archive"));
        }
        let origin = self
            .origin_for(&ArchiveOpenRequest {
                url: Some(url.to_string()),
                local_path: None,
                name: name.to_string(),
                service: String::new(),
                creator_id: String::new(),
                post_id: String::new(),
                post_text: None,
            })
            .await?;
        let source = self.connect(&origin).await?;
        let id = archive_id_for(&origin.source);
        let session = blocking(move || ArchiveSession::load(id, origin, source)).await?;
        Ok(session
            .entries
            .into_iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.path)
            .collect())
    }

    pub async fn open(&self, request: ArchiveOpenRequest) -> Result<ArchiveOpenResult, String> {
        if request.name.trim().is_empty() {
            return Err("The archive has no file name".to_string());
        }
        let post_candidates = request
            .post_text
            .as_deref()
            .map(extract_password_candidates)
            .unwrap_or_default();
        let origin = self.origin_for(&request).await?;
        let archive_id = archive_id_for(&origin.source);
        let format = detect_format(&origin.name);
        if format != "zip" {
            return Ok(ArchiveOpenResult {
                archive_id,
                format: format.to_string(),
                status: "unsupported".to_string(),
                encrypted: false,
                password_source: None,
                password_hints: post_candidates,
                folder: None,
            });
        }

        let session = self
            .session(&archive_id, Some(origin))
            .await
            .map_err(SessionError::message)?;
        if !session.is_encrypted() || session.password().is_some() {
            return Ok(session.result(Vec::new()));
        }

        let label = |password: &str, fallback: &'static str| {
            if post_candidates
                .iter()
                .any(|candidate| candidate == password)
            {
                "post"
            } else {
                fallback
            }
        };
        let mut ordered: Vec<(String, &'static str)> = Vec::new();
        if let Some(remembered) = self.passwords.password_for_archive(&archive_id)? {
            ordered.push((remembered.clone(), label(&remembered, "user")));
        }
        for password in self
            .passwords
            .creator_passwords(&request.service, &request.creator_id)?
        {
            let source = label(&password, "creator");
            ordered.push((password, source));
        }
        ordered.extend(
            post_candidates
                .iter()
                .map(|password| (password.clone(), "post")),
        );

        let mut tried = HashSet::new();
        for (password, source) in ordered {
            if !tried.insert(password.clone()) {
                continue;
            }
            if self.verify(&session, &password).await? {
                self.remember(&session, &request.service, &request.creator_id, &password);
                session.set_password(password.clone(), source);
                let hints = post_candidates
                    .into_iter()
                    .filter(|candidate| *candidate != password)
                    .collect();
                return Ok(session.result(hints));
            }
        }
        Ok(session.result(post_candidates))
    }

    pub async fn unlock(
        &self,
        archive_id: &str,
        password: &str,
    ) -> Result<ArchiveOpenResult, String> {
        let session = self
            .session(archive_id, None)
            .await
            .map_err(SessionError::message)?;
        if !session.is_encrypted() {
            return Ok(session.result(Vec::new()));
        }
        let mut attempts = vec![password.to_string()];
        if password.trim() != password {
            attempts.push(password.trim().to_string());
        }
        for attempt in attempts.into_iter().filter(|value| !value.is_empty()) {
            if self.verify(&session, &attempt).await? {
                let origin = session.origin.clone();
                self.remember(&session, &origin.service, &origin.creator_id, &attempt);
                session.set_password(attempt, "user");
                return Ok(session.result(Vec::new()));
            }
        }
        Err("wrong_password".to_string())
    }

    pub async fn serve_entry(
        &self,
        id: &str,
        index: usize,
        head: bool,
        range: Option<&str>,
    ) -> Result<Response, (StatusCode, String)> {
        let session = self.session(id, None).await.map_err(|error| match error {
            SessionError::Unknown => (StatusCode::NOT_FOUND, SessionError::Unknown.message()),
            SessionError::Failed(message) => (StatusCode::BAD_GATEWAY, message),
        })?;
        let entry = session
            .entries
            .get(index)
            .filter(|entry| !entry.is_dir)
            .cloned()
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Archive entry not found".to_string()))?;
        if entry.encrypted && session.password().is_none() {
            let remembered = self
                .passwords
                .password_for_archive(id)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            let Some(password) = remembered else {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "This archive is locked; enter its password first".to_string(),
                ));
            };
            session.set_password(password, "user");
        }

        let total = entry.size;
        let requested =
            range.and_then(|value| crate::server::axum_server::parse_range(value, total));
        let (start, end) = requested.unwrap_or((0, total.saturating_sub(1)));
        let mime = mime_guess::from_path(&entry.path)
            .first_or_octet_stream()
            .to_string();
        let partial = requested.is_some();
        if head || total == 0 {
            return entry_response(partial, start, end, total, &mime, Body::empty());
        }

        let body = if entry.stored && !entry.encrypted {
            let worker = session.clone();
            let data_start = blocking(move || worker.data_start(index))
                .await
                .map_err(|e| (StatusCode::BAD_GATEWAY, e))?;
            let (from, to) =
                stored_entry_range(data_start, total, start, end).ok_or_else(|| {
                    (
                        StatusCode::RANGE_NOT_SATISFIABLE,
                        "Requested range is outside the entry".to_string(),
                    )
                })?;
            match &session.source {
                SourceHandle::Remote(remote) => Self::passthrough(remote, from, to).await?,
                SourceHandle::Local { path, .. } => local_slice(path, from, to).await?,
            }
        } else {
            let job = session
                .extraction(index)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            let reader = job.attach();
            job.wait_started()
                .await
                .map_err(|e| (StatusCode::BAD_GATEWAY, e))?;
            job.body(reader, start, end)
        };
        entry_response(partial, start, end, total, &mime, body)
    }

    async fn passthrough(
        remote: &Arc<RemoteSource>,
        from: u64,
        to: u64,
    ) -> Result<Body, (StatusCode, String)> {
        let response = remote
            .open_stream(from, to + 1)
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
        Ok(Body::from_stream(response.bytes_stream()))
    }
}

async fn local_slice(path: &Path, from: u64, to: u64) -> Result<Body, (StatusCode, String)> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Archive file is gone: {e}")))?;
    file.seek(std::io::SeekFrom::Start(from))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Body::from_stream(
        tokio_util::io::ReaderStream::with_capacity(file.take(to - from + 1), PASSTHROUGH_CHUNK),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_stream_sources_drop_the_session_token() {
        assert_eq!(
            local_stream_source(
                "http://127.0.0.1:5555/cloud_stream/mega?folder_id=a&node_id=b&key=k&name=x%20y.zip&token=t"
            ),
            Some(Ok("/cloud_stream/mega?folder_id=a&node_id=b&key=k&name=x%20y.zip".to_string()))
        );
        assert_eq!(
            local_stream_source("/cloud_stream/mega?file_id=f&key=k&token=t"),
            Some(Ok("/cloud_stream/mega?file_id=f&key=k".to_string()))
        );
        assert!(matches!(
            local_stream_source("http://127.0.0.1:5555/cloud_stream/archive?id=1&entry=2"),
            Some(Err(_))
        ));
        assert_eq!(
            local_stream_source("https://file.pawchive.pw/data/a.zip"),
            None
        );
        assert_eq!(
            local_stream_source("http://192.168.1.5/cloud_stream/mega"),
            None
        );
    }

    fn entry(path: &str, is_dir: bool) -> EntryInfo {
        EntryInfo {
            path: path.to_string(),
            is_dir,
            size: if is_dir { 0 } else { 10 },
            compressed_size: 10,
            header_start: 0,
            stored: true,
            encrypted: false,
            aes: false,
        }
    }

    fn origin(source: &str) -> ArchiveOrigin {
        ArchiveOrigin {
            source: source.to_string(),
            local: true,
            name: "sample.zip".to_string(),
            service: "patreon".to_string(),
            creator_id: "1".to_string(),
            post_id: "2".to_string(),
        }
    }

    #[test]
    fn synthesizes_missing_folders_from_paths() {
        let entries = vec![
            entry("a/b/c.mov", false),
            entry("a/", true),
            entry("root.txt", false),
            entry("a/b/d.jpg", false),
            entry("empty/", true),
        ];
        let nodes = build_nodes("x", &entries);
        let summary: Vec<(String, Option<String>, bool)> = nodes
            .iter()
            .map(|node| (node.id.clone(), node.parent_id.clone(), node.is_folder))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("x:d:a".into(), None, true),
                ("x:d:a/b".into(), Some("x:d:a".into()), true),
                ("x:f:0".into(), Some("x:d:a/b".into()), false),
                ("x:f:2".into(), None, false),
                ("x:f:3".into(), Some("x:d:a/b".into()), false),
                ("x:d:empty".into(), None, true),
            ]
        );
        let video = &nodes[2];
        assert_eq!(video.name, "c.mov");
        assert_eq!(video.mime_type.as_deref(), Some("video/quicktime"));
        assert_eq!(
            video.stream_url.as_deref(),
            Some("/cloud_stream/archive?id=x&entry=0")
        );
        assert_eq!(video.download_url, video.stream_url);
    }

    #[test]
    fn normalizes_entry_paths() {
        assert_eq!(normalize_entry_path("a\\b\\c.txt"), "a/b/c.txt");
        assert_eq!(normalize_entry_path("/./a//../b/"), "a/b");
        assert_eq!(normalize_entry_path("../../etc/passwd"), "etc/passwd");
    }

    #[test]
    fn detects_formats_by_name() {
        for (name, format) in [
            ("Pack.ZIP", "zip"),
            ("comic.cbz", "zip"),
            ("set.part1.rar", "rar"),
            ("set.7z.001", "7z"),
            ("set.7z", "7z"),
            ("video.mp4", "unknown"),
        ] {
            assert_eq!(detect_format(name), format, "{name}");
        }
    }

    fn build_sample_zip(path: &Path) -> Vec<(&'static str, Vec<u8>)> {
        use zip::unstable::write::FileOptionsExt;
        use zip::write::SimpleFileOptions;

        let readme = b"hello archive ".repeat(500);
        let raw: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        let notes = b"aes secret notes ".repeat(40);
        let legacy = b"zipcrypto legacy text ".repeat(30);
        let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer.start_file("docs/readme.txt", deflated).unwrap();
        writer.write_all(&readme).unwrap();
        writer
            .start_file(
                "photos/2024/raw.bin",
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(&raw).unwrap();
        writer
            .start_file(
                "secret/notes.txt",
                deflated.with_aes_encryption(zip::AesMode::Aes256, "hunter2"),
            )
            .unwrap();
        writer.write_all(&notes).unwrap();
        writer
            .start_file(
                "secret/legacy.txt",
                deflated.with_deprecated_encryption(b"hunter2").unwrap(),
            )
            .unwrap();
        writer.write_all(&legacy).unwrap();
        writer.finish().unwrap();
        vec![
            ("docs/readme.txt", readme),
            ("photos/2024/raw.bin", raw),
            ("secret/notes.txt", notes),
            ("secret/legacy.txt", legacy),
        ]
    }

    #[test]
    fn local_zip_lists_validates_and_extracts() {
        let dir = std::env::temp_dir().join(format!("pawstash-archive-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.zip");
        let expected = build_sample_zip(&path);

        let source = SourceHandle::local(&path).unwrap();
        let session =
            ArchiveSession::load("test".to_string(), origin(&path.to_string_lossy()), source)
                .unwrap();
        let paths: Vec<&str> = session.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            paths,
            expected.iter().map(|(path, _)| *path).collect::<Vec<_>>()
        );
        assert!(session.is_encrypted());
        assert!(session.entries[2].aes && session.entries[2].encrypted);
        assert!(!session.entries[3].aes && session.entries[3].encrypted);
        assert!(session.entries[1].stored && !session.entries[0].stored);

        let result = session.result(Vec::new());
        assert_eq!(result.status, "password_required");
        let folder = result.folder.unwrap();
        assert_eq!(folder.total_files, 4);
        assert_eq!(folder.nodes.iter().filter(|node| node.is_folder).count(), 4);
        assert!(folder
            .nodes
            .iter()
            .any(|node| node.id == "test:f:2" && node.is_encrypted));

        assert!(!session.verify_password("wrong").unwrap());
        assert!(session.verify_password("hunter2").unwrap());
        assert!(session
            .extract_entry(2, &mut Vec::new(), |_| Ok(()))
            .is_err());
        session.set_password("hunter2".to_string(), "user");
        assert_eq!(session.result(Vec::new()).status, "ready");

        for (index, (name, content)) in expected.iter().enumerate() {
            let mut out = Vec::new();
            let written = session.extract_entry(index, &mut out, |_| Ok(())).unwrap();
            assert_eq!(written, content.len() as u64, "{name}");
            assert_eq!(&out, content, "{name}");
        }

        let data_start = session.data_start(1).unwrap();
        let (from, to) =
            stored_entry_range(data_start, expected[1].1.len() as u64, 100, 199).unwrap();
        let archive_bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            &archive_bytes[from as usize..=to as usize],
            &expected[1].1[100..200]
        );

        drop(session);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    const LIVE_ARCHIVE: &str = "https://file.pawchive.pw/data/56/55/565569f370906b543e1a64dbd70b0b3972da114f8ede7eb190be71da0b0bbf1b.zip";

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "hits the live Pawchive file host"]
    async fn live_remote_zip_lists_without_full_download() {
        let config = Arc::new(crate::config::ConfigManager::new().expect("settings"));
        let gateway = Arc::new(UpstreamGateway::new(config));
        let pacer = Arc::new(HostPacer::new(reader::HOST_REQUEST_INTERVAL));
        let url = reqwest::Url::parse(LIVE_ARCHIVE).unwrap();
        let remote = RemoteSource::open(Transport::Upstream(gateway), pacer, url)
            .await
            .expect("open");
        let source = SourceHandle::Remote(remote.clone());
        let session = tokio::task::spawn_blocking(move || {
            ArchiveSession::load("live".to_string(), origin(LIVE_ARCHIVE), source)
        })
        .await
        .unwrap()
        .expect("index");
        let listing_bytes = remote.fetched_bytes();

        assert_eq!(remote.len(), 239_489_339);
        assert_eq!(session.entries.len(), 11);
        assert_eq!(session.entries[0].path, "black holes");
        assert!(session.entries[0].is_dir);
        let files: Vec<&EntryInfo> = session
            .entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .collect();
        assert_eq!(files.len(), 10);
        assert!(files.iter().all(|entry| {
            entry.path.to_lowercase().ends_with(".mov") && entry.stored && !entry.encrypted
        }));
        assert!(
            listing_bytes < 1 << 20,
            "listing fetched {listing_bytes} bytes"
        );

        let session = Arc::new(session);
        let first_file = session
            .entries
            .iter()
            .position(|entry| !entry.is_dir)
            .unwrap();
        let worker = session.clone();
        let data_start = tokio::task::spawn_blocking(move || worker.data_start(first_file))
            .await
            .unwrap()
            .expect("data start");
        let response = remote
            .request_range(data_start, data_start + 12)
            .await
            .expect("entry head");
        let head = response.bytes().await.unwrap();
        assert!(matches!(
            &head[4..8],
            b"ftyp" | b"wide" | b"free" | b"mdat" | b"moov" | b"skip" | b"pnot"
        ));
    }
}
