use crate::api::models::Post;
use crate::archive::ArchiveRegistry;
use crate::cloud::{CloudFolderResult, CloudResolver};
use crate::config::settings::AppSettings;
use crate::db::content_index::{IndexCandidate, IndexedFormats};
use crate::AppState;
use std::collections::BTreeSet;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub const EVENT: &str = "post-contents-indexed";
const CLOUD_DOCUMENT: &str = "cloud_folder";
const FIRST_RUN_DELAY: Duration = Duration::from_secs(90);
const BUSY_RECHECK: Duration = Duration::from_secs(60);
const IDLE_INTERVAL: Duration = Duration::from_secs(30 * 60);
const NETWORK_PAUSE: Duration = Duration::from_secs(4);
const BATCH_SIZE: u32 = 25;
const MAX_SOURCES_PER_KIND: usize = 6;

struct ArchiveRef {
    name: String,
    path: String,
    server: Option<String>,
}

struct Sources {
    archives: Vec<ArchiveRef>,
    clouds: Vec<String>,
    fingerprint: String,
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_RUN_DELAY).await;
        loop {
            let pause = run_batch(&app).await.unwrap_or_else(|error| {
                tracing::warn!(%error, "Content index pass failed");
                BUSY_RECHECK
            });
            tokio::time::sleep(pause).await;
        }
    });
}

fn busy(state: &AppState) -> bool {
    state.download_manager.is_busy() || state.sync_manager.is_syncing()
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

async fn run_batch(app: &AppHandle) -> Result<Duration, String> {
    let Some(state) = app.try_state::<AppState>() else {
        return Ok(BUSY_RECHECK);
    };
    let settings = state.config_manager.load()?;
    if !settings.content_indexing_enabled {
        return Ok(IDLE_INTERVAL);
    }
    if busy(&state) {
        return Ok(BUSY_RECHECK);
    }
    let repository = state.content_index.clone();
    let candidates = blocking(move || repository.candidates(BATCH_SIZE)).await?;
    if candidates.is_empty() {
        return Ok(IDLE_INTERVAL);
    }
    let resolver = CloudResolver::new(&settings)?;
    for candidate in candidates {
        if busy(&state) {
            return Ok(BUSY_RECHECK);
        }
        if let Some(indexed) = index_post(&state, &settings, &resolver, candidate).await? {
            if let Err(error) = app.emit(EVENT, &indexed) {
                tracing::debug!(%error, "Could not announce indexed post contents");
            }
        }
    }
    Ok(Duration::ZERO)
}

async fn index_post(
    state: &AppState,
    settings: &AppSettings,
    resolver: &CloudResolver,
    candidate: IndexCandidate,
) -> Result<Option<IndexedFormats>, String> {
    let post = Post::from_json_str(&candidate.snapshot_json).ok();
    let sources = post
        .as_ref()
        .map(|post| sources_of(post, settings.cloud_scraping_enabled));
    let fingerprint = sources
        .as_ref()
        .map_or_else(|| "unreadable".to_string(), |s| s.fingerprint.clone());

    let repository = state.content_index.clone();
    if candidate.complete && candidate.sources.as_deref() == Some(fingerprint.as_str()) {
        blocking(move || repository.touch(&candidate)).await?;
        return Ok(None);
    }
    let (Some(post), Some(sources)) = (post, sources) else {
        blocking(move || repository.record(&candidate, &fingerprint, &[], true)).await?;
        return Ok(None);
    };

    let mut names = Vec::new();
    let mut complete = true;
    let provider_id = post.extra.get("provider_id").and_then(|v| v.as_str());
    for archive in &sources.archives {
        let url = state
            .provider_manager
            .resolve_media_url(
                &post.service,
                &archive.path,
                archive.server.as_deref(),
                provider_id,
            )
            .await;
        match state.archives.list_entry_names(&url, &archive.name).await {
            Ok(entries) => names.extend(entries),
            Err(error) => {
                complete = false;
                tracing::debug!(post = %post.id, archive = %archive.name, %error, "Could not list archive for the content index");
            }
        }
        tokio::time::sleep(NETWORK_PAUSE).await;
    }
    for url in &sources.clouds {
        match cloud_file_names(state, resolver, url).await {
            Ok((files, fetched)) => {
                names.extend(files);
                if fetched {
                    tokio::time::sleep(NETWORK_PAUSE).await;
                }
            }
            Err(error) => {
                complete = false;
                tracing::debug!(post = %post.id, %url, %error, "Could not list cloud folder for the content index");
            }
        }
    }

    let formats = formats_of(&names);
    let indexed = IndexedFormats {
        service: candidate.service.clone(),
        creator_id: candidate.creator_id.clone(),
        post_id: candidate.post_id.clone(),
        formats: formats.clone(),
    };
    blocking(move || repository.record(&candidate, &fingerprint, &formats, complete)).await?;
    Ok((!indexed.formats.is_empty()).then_some(indexed))
}

async fn cloud_file_names(
    state: &AppState,
    resolver: &CloudResolver,
    url: &str,
) -> Result<(Vec<String>, bool), String> {
    let content = state.content.clone();
    let key = url.to_string();
    let cached =
        blocking(move || content.load_document::<CloudFolderResult>(CLOUD_DOCUMENT, &key, "", ""))
            .await?;
    let (folder, fetched) = match cached {
        Some(folder) => (folder, false),
        None => {
            let folder = resolver.resolve(url).await?;
            let content = state.content.clone();
            let key = url.to_string();
            let stored = folder.clone();
            blocking(move || content.save_document(CLOUD_DOCUMENT, &key, "", "", &stored)).await?;
            (folder, true)
        }
    };
    let names = folder
        .nodes
        .into_iter()
        .filter(|node| !node.is_folder)
        .map(|node| node.name)
        .collect();
    Ok((names, fetched))
}

fn sources_of(post: &Post, cloud_enabled: bool) -> Sources {
    let files_unknown =
        post.file.is_none() && post.attachments.is_none() && post.detail_fetched != Some(true);
    let archives: Vec<ArchiveRef> = post
        .file
        .iter()
        .chain(post.attachments.iter().flatten())
        .filter_map(|item| {
            let path = item.path.clone()?;
            let name = item
                .name
                .clone()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| path.rsplit('/').next().unwrap_or_default().to_string());
            ArchiveRegistry::can_list(&name).then(|| ArchiveRef {
                name,
                path,
                server: item.server.clone(),
            })
        })
        .take(MAX_SOURCES_PER_KIND)
        .collect();
    let clouds: Vec<String> = if cloud_enabled {
        post.cloud_urls
            .iter()
            .filter(|url| {
                !crate::cloud::mega::is_protected_link(url)
                    && !crate::cloud::mega::is_keyless_link(url)
            })
            .take(MAX_SOURCES_PER_KIND)
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let mut parts: Vec<String> = archives
        .iter()
        .map(|archive| format!("a:{}", archive.path))
        .chain(clouds.iter().map(|url| format!("c:{url}")))
        .collect();
    parts.sort();
    if files_unknown {
        parts.push("pending".to_string());
    }
    Sources {
        archives,
        clouds,
        fingerprint: parts.join("\n"),
    }
}

fn kind_of(name: &str) -> Option<&'static str> {
    let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
    match extension.as_str() {
        "mp4" | "webm" | "mkv" | "mov" | "avi" | "m4v" | "wmv" | "flv" => Some("video"),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "avif" | "bmp" => Some("image"),
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "opus" | "aac" => Some("audio"),
        _ => None,
    }
}

fn formats_of(names: &[String]) -> Vec<String> {
    names
        .iter()
        .filter_map(|name| kind_of(name))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::Attachment;

    fn attachment(name: &str, path: &str) -> Attachment {
        Attachment {
            name: Some(name.into()),
            path: Some(path.into()),
            ..Attachment::default()
        }
    }

    #[test]
    fn classifies_entry_names() {
        let names = ["Outfit 1/Cam 2.MP4", "cover.jpg", "readme.txt", "noext"]
            .map(String::from)
            .to_vec();
        assert_eq!(formats_of(&names), vec!["image", "video"]);
    }

    #[test]
    fn collects_zip_attachments_and_cloud_links() {
        let post: Post = serde_json::from_value(serde_json::json!({
            "id": "1", "user": "u", "service": "patreon",
            "attachments": [],
            "cloud_urls": ["https://app.filen.io/#/f/x%23y"]
        }))
        .unwrap();
        let mut post = post;
        post.attachments = Some(vec![
            attachment("pack.zip", "/aa/bb/pack.zip"),
            attachment("cover.jpg", "/aa/bb/cover.jpg"),
            attachment("pack.rar", "/aa/bb/pack.rar"),
        ]);
        let sources = sources_of(&post, true);
        assert_eq!(sources.archives.len(), 1);
        assert_eq!(sources.archives[0].name, "pack.zip");
        assert_eq!(sources.clouds.len(), 1);
        assert_eq!(
            sources.fingerprint,
            "a:/aa/bb/pack.zip\nc:https://app.filen.io/#/f/x%23y"
        );
        assert!(sources_of(&post, false).clouds.is_empty());
    }
}
