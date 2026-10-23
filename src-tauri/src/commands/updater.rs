use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;
use tokio::io::AsyncWriteExt;

const GITHUB_REPO: &str = "pawstash/pawstash";
const MAX_UPDATE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const CANCELLED: &str = "UPDATE_CANCELLED";
const USER_AGENT: &str = concat!(
    "Pawstash/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/pawstash/pawstash)"
);

static CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub content_type: Option<String>,
    #[serde(default)]
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub prerelease: bool,
    pub published_at: Option<String>,
    pub html_url: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub is_prerelease: bool,
    pub release_name: String,
    pub release_notes: String,
    pub published_at: String,
    pub release_url: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub asset_size: Option<u64>,
    #[serde(default)]
    pub contributors: Vec<Contributor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contributor {
    pub login: String,
    pub avatar_url: String,
    pub html_url: String,
    pub commits: u32,
}

#[derive(Deserialize)]
struct CompareResponse {
    commits: Vec<CompareCommit>,
}

#[derive(Deserialize)]
struct CompareCommit {
    author: Option<CommitAuthor>,
}

#[derive(Deserialize)]
struct CommitAuthor {
    login: String,
    avatar_url: String,
    html_url: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProgressPayload {
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f64,
    pub speed_bytes_per_sec: u64,
}

fn strip_redundant_release_heading(body: &str) -> String {
    let mut output = Vec::new();
    let mut stripped_first_heading = false;

    for line in body.lines() {
        let trimmed = line.trim();
        if !stripped_first_heading && trimmed.starts_with('#') {
            let heading_text = trimmed.trim_start_matches('#').trim().to_lowercase();
            if heading_text.starts_with("changes") || heading_text.starts_with("what's") {
                stripped_first_heading = true;
                continue;
            }
        }
        output.push(line);
    }
    output.join("\n").trim().to_string()
}

fn pending_releases<'a>(
    releases: &'a [GitHubRelease],
    current_version: &str,
    include_prereleases: bool,
) -> Vec<&'a GitHubRelease> {
    let allow_prereleases = include_prereleases || current_version.contains('-');
    let newer: Vec<&GitHubRelease> = releases
        .iter()
        .filter(|r| allow_prereleases || !r.prerelease)
        .filter(|r| is_version_newer(r.tag_name.trim_start_matches('v'), current_version))
        .collect();
    let latest_is_prerelease = newer.first().is_some_and(|r| r.prerelease);
    newer
        .into_iter()
        .filter(|r| latest_is_prerelease || !r.prerelease)
        .collect()
}

fn combined_release_notes(releases: &[&GitHubRelease]) -> String {
    releases
        .iter()
        .filter_map(|r| {
            let title = r.name.as_deref().unwrap_or(&r.tag_name);
            let cleaned = strip_redundant_release_heading(r.body.as_deref().unwrap_or("").trim());
            match (cleaned.is_empty(), releases.len()) {
                (true, 1) => Some(title.to_string()),
                (true, _) => None,
                (false, _) => Some(format!("### {title}\n{cleaned}")),
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[tauri::command]
pub async fn check_for_updates(include_prereleases: bool) -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let client = crate::net::builder_proxied()
        .user_agent(USER_AGENT)
        .timeout(crate::net::defaults().request_timeout)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let releases = fetch_releases(&client).await?;

    let pending = pending_releases(&releases, &current_version, include_prereleases);

    if let Some(latest) = pending.first().copied() {
        let latest_tag = latest.tag_name.trim_start_matches('v');
        let (asset_url, asset_name, asset_size) = find_platform_asset(&latest.assets);
        let combined_notes = combined_release_notes(&pending);

        let installed_tag = releases
            .iter()
            .find(|r| r.tag_name.trim_start_matches('v') == current_version)
            .map(|r| r.tag_name.as_str());
        let contributors = match installed_tag {
            Some(base) => fetch_contributors(&client, base, &latest.tag_name)
                .await
                .unwrap_or_else(|error| {
                    tracing::warn!(%error, "Couldn't list the update's contributors");
                    Vec::new()
                }),
            None => Vec::new(),
        };

        return Ok(UpdateInfo {
            available: true,
            current_version: current_version.clone(),
            latest_version: latest_tag.to_string(),
            is_prerelease: latest.prerelease,
            release_name: latest
                .name
                .clone()
                .unwrap_or_else(|| latest.tag_name.clone()),
            release_notes: combined_notes,
            published_at: latest.published_at.clone().unwrap_or_default(),
            release_url: latest.html_url.clone(),
            download_url: asset_url,
            asset_name,
            asset_size,
            contributors,
        });
    }

    Ok(UpdateInfo {
        available: false,
        current_version: current_version.clone(),
        latest_version: current_version,
        is_prerelease: false,
        release_name: String::new(),
        release_notes: String::new(),
        published_at: String::new(),
        release_url: format!("https://github.com/{GITHUB_REPO}/releases"),
        download_url: None,
        asset_name: None,
        asset_size: None,
        contributors: Vec::new(),
    })
}

fn get_update_temp_dir() -> Result<std::path::PathBuf, String> {
    #[cfg(target_os = "android")]
    {
        if let Ok(path) = crate::commands::with_android_context(|env, context| {
            if let Ok(val) = env.call_method(context, "getCacheDir", "()Ljava/io/File;", &[]) {
                if let Ok(file_obj) = val.l() {
                    if !file_obj.is_null() {
                        if let Ok(path_val) = env.call_method(
                            &file_obj,
                            "getAbsolutePath",
                            "()Ljava/lang/String;",
                            &[],
                        ) {
                            if let Ok(path_obj) = path_val.l() {
                                let jstr: jni::objects::JString = path_obj.into();
                                let owned = env
                                    .get_string(&jstr)
                                    .map(|s| s.to_string_lossy().to_string());
                                if let Ok(path_string) = owned {
                                    return Ok(std::path::PathBuf::from(path_string));
                                }
                            }
                        }
                    }
                }
            }
            Ok(crate::db::storage::android_cache_dir())
        }) {
            return Ok(path);
        }
        Ok(crate::db::storage::android_cache_dir())
    }

    #[cfg(all(target_os = "linux", not(target_os = "android")))]
    {
        if let Some(cache_dir) = dirs::cache_dir() {
            let update_dir = cache_dir.join("pawstash").join("updates");
            if std::fs::create_dir_all(&update_dir).is_ok() {
                return Ok(update_dir);
            }
        }
        Ok(std::env::temp_dir())
    }

    #[cfg(not(any(target_os = "android", target_os = "linux")))]
    {
        Ok(std::env::temp_dir())
    }
}

#[tauri::command]
pub fn cancel_update_download() {
    CANCEL_REQUESTED.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn download_and_install_update(
    app_handle: tauri::AppHandle,
    download_url: String,
    asset_name: String,
) -> Result<(), String> {
    let parsed_url =
        reqwest::Url::parse(&download_url).map_err(|e| format!("Invalid download URL: {e}"))?;
    if !is_official_asset_url(&parsed_url) {
        return Err("Update URL is not an official Pawstash release asset".to_string());
    }

    let temp_dir = get_update_temp_dir()?;
    let safe_name = if asset_name.trim().is_empty() {
        "pawstash-update".to_string()
    } else {
        std::path::Path::new(&asset_name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "pawstash-update".to_string())
    };
    let target_path = temp_dir.join(&safe_name);
    CANCEL_REQUESTED.store(false, Ordering::SeqCst);

    let client = crate::net::builder_proxied()
        .user_agent(USER_AGENT)
        .read_timeout(crate::net::defaults().request_timeout)
        .no_gzip()
        .build()
        .map_err(|e| format!("HTTP Client error: {e}"))?;

    let releases = fetch_releases(&client).await?;
    let current_version = env!("CARGO_PKG_VERSION");
    let verified_asset = releases
        .iter()
        .filter(|release| is_version_newer(&release.tag_name, current_version))
        .find_map(|release| {
            let platform_asset = find_platform_asset_ref(&release.assets)?;
            (platform_asset.name == safe_name
                && platform_asset.browser_download_url == download_url)
                .then_some(platform_asset)
        })
        .ok_or_else(|| {
            "Update asset is not the current platform asset from a newer Pawstash release"
                .to_string()
        })?;

    if verified_asset.size == 0 || verified_asset.size > MAX_UPDATE_BYTES {
        return Err("Update asset has an invalid declared size".to_string());
    }
    let expected_digest = verified_asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .filter(|digest| digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| "GitHub did not provide a valid SHA-256 digest for this asset".to_string())?
        .to_lowercase();

    let response = client
        .get(parsed_url.clone())
        .send()
        .await
        .map_err(|e| format!("Download request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with HTTP status: {}",
            response.status()
        ));
    }

    let total_bytes = verified_asset.size;
    if let Some(response_size) = response.content_length() {
        if response_size != total_bytes {
            return Err(format!(
                "Update size differs from GitHub release metadata: expected {total_bytes}, got {response_size}"
            ));
        }
    }
    let mut file = tokio::fs::File::create(&target_path)
        .await
        .map_err(|e| format!("Failed to create temp update file: {e}"))?;

    let downloaded = async {
        let mut stream = response.bytes_stream();
        let mut downloaded_bytes: u64 = 0;
        let mut last_emit = std::time::Instant::now();
        let mut last_bytes: u64 = 0;
        let mut hasher = Sha256::new();

        while let Some(chunk_res) = futures_util::StreamExt::next(&mut stream).await {
            if CANCEL_REQUESTED.swap(false, Ordering::SeqCst) {
                return Err(CANCELLED.to_string());
            }
            let chunk = chunk_res.map_err(|e| format!("Error during stream chunk read: {e}"))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write chunk: {e}"))?;

            downloaded_bytes += chunk.len() as u64;
            if downloaded_bytes > total_bytes {
                return Err("Downloaded update exceeds its declared size".to_string());
            }
            hasher.update(&chunk);

            if last_emit.elapsed().as_millis() >= 100
                || (total_bytes > 0 && downloaded_bytes >= total_bytes)
            {
                let elapsed_sec = last_emit.elapsed().as_secs_f64();
                let speed = if elapsed_sec > 0.0 {
                    ((downloaded_bytes.saturating_sub(last_bytes)) as f64 / elapsed_sec) as u64
                } else {
                    0
                };

                let percentage = if total_bytes > 0 {
                    ((downloaded_bytes as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0)
                } else {
                    0.0
                };

                if let Err(error) = app_handle.emit(
                    "update-download-progress",
                    UpdateProgressPayload {
                        downloaded: downloaded_bytes,
                        total: total_bytes,
                        percentage,
                        speed_bytes_per_sec: speed,
                    },
                ) {
                    tracing::warn!("Failed to emit update progress: {error}");
                }

                last_emit = std::time::Instant::now();
                last_bytes = downloaded_bytes;
            }
        }

        file.flush()
            .await
            .map_err(|e| format!("Failed to flush downloaded file: {e}"))?;
        drop(file);

        let meta = tokio::fs::metadata(&target_path)
            .await
            .map_err(|e| format!("Downloaded file missing: {e}"))?;
        if meta.len() != total_bytes {
            return Err(format!(
                "Downloaded update size mismatch: expected {total_bytes}, got {}",
                meta.len()
            ));
        }
        let actual_digest = format!("{:x}", hasher.finalize());
        if actual_digest != expected_digest {
            return Err("Downloaded update failed SHA-256 verification".to_string());
        }
        Ok::<(), String>(())
    }
    .await;
    if let Err(error) = downloaded {
        if let Err(cleanup) = tokio::fs::remove_file(&target_path).await {
            tracing::warn!(%cleanup, "Couldn't remove the unfinished update download");
        }
        return Err(error);
    }

    #[cfg(target_os = "android")]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o644));
    }

    launch_installer_and_exit(app_handle, &target_path).await?;

    Ok(())
}

async fn launch_installer_and_exit(
    app_handle: tauri::AppHandle,
    target_path: &std::path::Path,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let ext = target_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "exe" {
            std::process::Command::new(target_path)
                .spawn()
                .map_err(|e| format!("Failed to launch Windows installer: {e}"))?;
            app_handle.exit(0);
            Ok(())
        } else if ext == "msi" {
            std::process::Command::new("msiexec")
                .args(["/i", &target_path.to_string_lossy()])
                .spawn()
                .map_err(|e| format!("Failed to launch MSI installer: {e}"))?;
            app_handle.exit(0);
            Ok(())
        } else {
            Err(format!(
                "Unsupported Windows update asset extension: .{ext}"
            ))
        }
    }

    #[cfg(target_os = "android")]
    {
        let _ = app_handle;
        install_package_android(target_path)?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let _ = app_handle;
        std::process::Command::new("open")
            .arg(target_path)
            .spawn()
            .map_err(|e| format!("Failed to open macOS installer: {e}"))?;
        return Ok(());
    }

    #[cfg(all(target_os = "linux", not(target_os = "android")))]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(target_path, std::fs::Permissions::from_mode(0o755));

        let is_sandboxed =
            std::path::Path::new("/.flatpak-info").exists() || std::env::var_os("SNAP").is_some();
        if is_sandboxed {
            return crate::linux_open::open(target_path)
                .await
                .map_err(|e| format!("Failed to open downloaded update: {e}"));
        }

        let ext = target_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "appimage" {
            if let Ok(appimage_env) = std::env::var("APPIMAGE") {
                let current_appimage = std::path::PathBuf::from(&appimage_env);
                if current_appimage.exists() && current_appimage != *target_path {
                    let tmp_dest = current_appimage.with_extension("appimage.update_tmp");
                    let _ = std::fs::remove_file(&tmp_dest);
                    if std::fs::copy(target_path, &tmp_dest).is_ok() {
                        let _ = std::fs::set_permissions(
                            &tmp_dest,
                            std::fs::Permissions::from_mode(0o755),
                        );
                        if std::fs::rename(&tmp_dest, &current_appimage).is_ok() {
                            let _ = std::fs::remove_file(target_path);
                            let mut cmd = std::process::Command::new(&current_appimage);
                            crate::linux_open::apply_host_env(&mut cmd);
                            cmd.spawn()
                                .map_err(|e| format!("Failed to launch AppImage: {e}"))?;
                            app_handle.exit(0);
                            return Ok(());
                        }
                        let _ = std::fs::remove_file(&tmp_dest);
                    }
                }
            }

            let mut cmd = std::process::Command::new(target_path);
            crate::linux_open::apply_host_env(&mut cmd);
            cmd.spawn()
                .map_err(|e| format!("Failed to launch AppImage: {e}"))?;
            app_handle.exit(0);
            return Ok(());
        } else if ext == "deb" {
            let deb_str = target_path.to_string_lossy().to_string();
            let current_exe = std::env::current_exe().ok();
            let exe_str = current_exe
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "/usr/bin/pawstash".to_string());

            let has_apt = std::path::Path::new("/usr/bin/apt-get").exists();
            let has_pkexec = std::path::Path::new("/usr/bin/pkexec").exists();

            if has_apt && has_pkexec {
                let script = format!(
                    "if pkexec apt-get install -y --reinstall \"{}\"; then \"{}\"; else \"{}\"; fi",
                    deb_str, exe_str, exe_str
                );
                std::process::Command::new("sh")
                    .args(["-c", &script])
                    .spawn()
                    .map_err(|e| format!("Failed to launch package installer: {e}"))?;
                app_handle.exit(0);
                return Ok(());
            } else {
                return crate::linux_open::open(target_path)
                    .await
                    .map_err(|e| format!("Failed to open Linux package: {e}"));
            }
        } else {
            return crate::linux_open::open(target_path)
                .await
                .map_err(|e| format!("Failed to open Linux package: {e}"));
        }
    }

    #[cfg(not(any(
        target_os = "windows",
        target_os = "android",
        target_os = "macos",
        target_os = "linux"
    )))]
    {
        let _ = app_handle;
        let _ = target_path;
        Err("Unsupported operating system for auto-install".to_string())
    }
}

#[cfg(target_os = "android")]
fn install_package_android(apk_path: &std::path::Path) -> Result<(), String> {
    let apk_path_str = apk_path.to_string_lossy().to_string();
    crate::commands::with_android_context(|env, context| {
        let jstr = env
            .new_string(&apk_path_str)
            .map_err(|e| format!("New string error: {e}"))?;
        env.call_method(
            context,
            "installApk",
            "(Ljava/lang/String;)V",
            &[jni::objects::JValue::Object(&jstr)],
        )
        .map_err(|e| format!("Failed to call installApk: {e}"))?;
        Ok(())
    })
}

fn find_platform_asset(assets: &[ReleaseAsset]) -> (Option<String>, Option<String>, Option<u64>) {
    if let Some(asset) = find_platform_asset_ref(assets) {
        return (
            Some(asset.browser_download_url.clone()),
            Some(asset.name.clone()),
            Some(asset.size),
        );
    }
    (None, None, None)
}

fn find_platform_asset_ref(assets: &[ReleaseAsset]) -> Option<&ReleaseAsset> {
    #[cfg(target_os = "windows")]
    {
        if let Some(asset) = assets.iter().find(|a| {
            let name = a.name.to_lowercase();
            name.ends_with("-setup.exe")
                || name.ends_with("_setup.exe")
                || name == "pawstash-setup.exe"
        }) {
            return Some(asset);
        }
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".msi"))
        {
            return Some(asset);
        }
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".exe"))
        {
            return Some(asset);
        }
    }

    #[cfg(target_os = "android")]
    {
        let preferred_pattern = if cfg!(target_arch = "aarch64") {
            Some("arm64")
        } else if cfg!(target_arch = "arm") {
            Some("v7a")
        } else if cfg!(target_arch = "x86_64") {
            Some("x86_64")
        } else {
            None
        };

        if let Some(pattern) = preferred_pattern {
            if let Some(asset) = assets.iter().find(|a| {
                let name = a.name.to_lowercase();
                name.ends_with(".apk")
                    && (name.contains(pattern) || (pattern == "v7a" && name.contains("armeabi")))
            }) {
                return Some(asset);
            }
        }

        if let Some(asset) = assets.iter().find(|a| {
            let name = a.name.to_lowercase();
            name == "pawstash.apk" || name == "pawstash-universal.apk"
        }) {
            return Some(asset);
        }

        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".apk"))
        {
            return Some(asset);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".dmg"))
        {
            return Some(asset);
        }
        if let Some(asset) = assets.iter().find(|a| {
            a.name.to_lowercase().ends_with(".app.tar.gz")
                || a.name.to_lowercase().ends_with(".tar.gz")
        }) {
            return Some(asset);
        }
    }

    #[cfg(all(target_os = "linux", not(target_os = "android")))]
    {
        let is_sandboxed =
            std::path::Path::new("/.flatpak-info").exists() || std::env::var_os("SNAP").is_some();
        let is_running_appimage =
            std::env::var_os("APPIMAGE").is_some() || std::env::var_os("APPDIR").is_some();

        let is_deb_system = !is_sandboxed
            && !is_running_appimage
            && std::path::Path::new("/etc/debian_version").exists()
            && (std::path::Path::new("/usr/bin/dpkg").exists()
                || std::path::Path::new("/usr/bin/apt-get").exists());

        if is_deb_system {
            if let Some(asset) = assets
                .iter()
                .find(|a| a.name.to_lowercase().ends_with(".deb"))
            {
                return Some(asset);
            }
        }

        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.ends_with(".AppImage") || a.name.to_lowercase().ends_with(".appimage"))
        {
            return Some(asset);
        }

        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".deb"))
        {
            return Some(asset);
        }
    }

    None
}

async fn fetch_releases(client: &reqwest::Client) -> Result<Vec<GitHubRelease>, String> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases");
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch releases from GitHub: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }
    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub releases response: {e}"))
}

async fn fetch_contributors(
    client: &reqwest::Client,
    base: &str,
    head: &str,
) -> Result<Vec<Contributor>, String> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/compare/{base}...{head}");
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| format!("Failed to compare releases on GitHub: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }
    let compare: CompareResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub compare response: {e}"))?;

    let mut contributors: Vec<Contributor> = Vec::new();
    for author in compare
        .commits
        .into_iter()
        .filter_map(|commit| commit.author)
    {
        if author.kind == "Bot" {
            continue;
        }
        match contributors.iter_mut().find(|c| c.login == author.login) {
            Some(existing) => existing.commits += 1,
            None => contributors.push(Contributor {
                login: author.login,
                avatar_url: author.avatar_url,
                html_url: author.html_url,
                commits: 1,
            }),
        }
    }
    contributors.sort_by(|a, b| b.commits.cmp(&a.commits));
    Ok(contributors)
}

fn is_official_asset_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url
            .path()
            .starts_with(&format!("/{GITHUB_REPO}/releases/download/"))
}

fn parse_semver(tag: &str) -> (Vec<u64>, Option<String>) {
    let clean = tag.trim().trim_start_matches('v');
    let parts: Vec<&str> = clean.splitn(2, '-').collect();
    let nums: Vec<u64> = parts[0]
        .split('.')
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();
    let pre = if parts.len() > 1 {
        Some(parts[1].to_string())
    } else {
        None
    };
    (nums, pre)
}

fn compare_prerelease_tokens(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();

    for (p1, p2) in a_parts.iter().zip(b_parts.iter()) {
        if p1 == p2 {
            continue;
        }
        if let (Ok(n1), Ok(n2)) = (p1.parse::<u64>(), p2.parse::<u64>()) {
            if n1 != n2 {
                return n1.cmp(&n2);
            }
        } else {
            let p1_prefix = p1.trim_end_matches(|c: char| c.is_ascii_digit());
            let p2_prefix = p2.trim_end_matches(|c: char| c.is_ascii_digit());
            if p1_prefix == p2_prefix {
                let n1 = p1[p1_prefix.len()..].parse::<u64>().ok();
                let n2 = p2[p2_prefix.len()..].parse::<u64>().ok();
                if let (Some(num1), Some(num2)) = (n1, n2) {
                    if num1 != num2 {
                        return num1.cmp(&num2);
                    }
                }
            }
            return p1.cmp(p2);
        }
    }
    a_parts.len().cmp(&b_parts.len())
}

pub(crate) fn is_version_newer(latest: &str, current: &str) -> bool {
    let (latest_nums, latest_pre) = parse_semver(latest);
    let (current_nums, current_pre) = parse_semver(current);

    let max_len = latest_nums.len().max(current_nums.len());
    for i in 0..max_len {
        let l = latest_nums.get(i).copied().unwrap_or(0);
        let c = current_nums.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    match (latest_pre, current_pre) {
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (Some(l), Some(c)) => compare_prerelease_tokens(&l, &c) == std::cmp::Ordering::Greater,
        (None, None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, prerelease: bool, change: &str) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag.to_string(),
            name: Some(tag.to_string()),
            body: Some(format!("### Changes in {tag}\n\n- {change}")),
            prerelease,
            published_at: None,
            html_url: String::new(),
            assets: Vec::new(),
        }
    }

    fn published() -> Vec<GitHubRelease> {
        vec![
            release("v26.10.1-7", true, "fix: imported posts stuck"),
            release("v26.10.1-6", true, "feat: post download controls"),
            release("v26.10.1-5", true, "fix: Linux/macOS build"),
            release("v26.10.1-3", true, "feat: ui redesign"),
            release("v26.9.4", false, "fix(updater): atomic AppImage update"),
            release("v26.9.3", false, "perf(linux): drop in-scroll blurs"),
        ]
    }

    #[test]
    fn notes_cover_every_release_since_the_installed_one() {
        let releases = published();
        let pending = pending_releases(&releases, "26.10.1-3", false);
        let tags: Vec<&str> = pending.iter().map(|r| r.tag_name.as_str()).collect();
        assert_eq!(tags, ["v26.10.1-7", "v26.10.1-6", "v26.10.1-5"]);
        let notes = combined_release_notes(&pending);
        for change in [
            "imported posts stuck",
            "post download controls",
            "Linux/macOS build",
        ] {
            assert!(notes.contains(change), "{change} missing from {notes}");
        }
        assert!(!notes.contains("ui redesign"));
    }

    #[test]
    fn stable_installs_skip_prereleases_unless_asked() {
        let releases = published();
        let stable: Vec<&str> = pending_releases(&releases, "26.9.3", false)
            .iter()
            .map(|r| r.tag_name.as_str())
            .collect();
        assert_eq!(stable, ["v26.9.4"]);
        assert_eq!(pending_releases(&releases, "26.9.3", true).len(), 5);
    }

    #[test]
    fn version_comparison_orders_releases_and_prereleases() {
        for (newer, older) in [
            ("0.2.0", "0.1.0"),
            ("0.1.1", "0.1.0"),
            ("1.0.0", "0.9.9"),
            ("0.1.0", "0.0.9"),
            ("0.1.0", "0.1.0-beta.1"),
            ("26.8.1", "26.8.1-pre1"),
            ("26.8.2", "26.8.1-pre1"),
            ("v26.8.2", "26.8.1"),
            ("26.8.1-pre2", "26.8.1-pre1"),
            ("26.8.1-pre10", "26.8.1-pre2"),
            ("26.8.1-beta.2", "26.8.1-beta.1"),
            ("26.8.1-beta.10", "26.8.1-beta.2"),
            ("26.8.2-5", "26.8.2-1"),
            ("26.8.2", "26.8.2-5"),
            ("26.8.3-1", "26.8.2"),
        ] {
            assert!(is_version_newer(newer, older), "{newer} > {older}");
            assert!(!is_version_newer(older, newer), "{older} < {newer}");
        }
        for same in ["0.1.0", "26.8.1-pre1"] {
            assert!(!is_version_newer(same, same), "{same}");
        }
    }

    #[test]
    fn picks_the_installer_for_this_platform() {
        let assets: Vec<ReleaseAsset> = [
            "Pawstash_26.8.1_x64-setup.exe",
            "pawstash_26.8.1_universal.apk",
            "Pawstash_26.8.1_universal.dmg",
            "pawstash_26.8.1_amd64.AppImage",
            "pawstash_26.8.1_amd64.deb",
        ]
        .into_iter()
        .map(|name| ReleaseAsset {
            name: name.to_string(),
            browser_download_url: format!(
                "https://github.com/pawstash/pawstash/releases/download/v26.8.1/{name}"
            ),
            size: 1,
            content_type: None,
            digest: None,
        })
        .collect();

        let (url, name, size) = find_platform_asset(&assets);
        let name = name.expect("an asset for this platform");
        assert!(url.unwrap().ends_with(&name));
        assert_eq!(size, Some(1));
        let expected: &[&str] = if cfg!(target_os = "windows") {
            &[".exe"]
        } else if cfg!(target_os = "android") {
            &[".apk"]
        } else if cfg!(target_os = "macos") {
            &[".dmg"]
        } else {
            &[".AppImage", ".deb"]
        };
        assert!(expected.iter().any(|ext| name.ends_with(ext)), "{name}");
    }
}
