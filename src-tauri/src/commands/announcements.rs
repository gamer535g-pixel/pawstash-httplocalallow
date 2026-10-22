use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::updater::is_version_newer;

const FEED_URL: &str =
    "https://raw.githubusercontent.com/pawstash/pawstash/meta/announcements.json";
const MAX_FEED_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnouncementAction {
    pub label: HashMap<String, String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Announcement {
    pub id: String,
    #[serde(default = "default_severity")]
    pub severity: String,
    pub title: HashMap<String, String>,
    #[serde(default)]
    pub body: HashMap<String, String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub min_version: Option<String>,
    #[serde(default)]
    pub max_version: Option<String>,
    #[serde(default)]
    pub starts_at: Option<String>,
    #[serde(default)]
    pub ends_at: Option<String>,
    #[serde(default)]
    pub actions: Vec<AnnouncementAction>,
}

#[derive(Debug, Deserialize)]
struct AnnouncementFeed {
    #[serde(default)]
    announcements: Vec<Announcement>,
}

fn default_severity() -> String {
    "info".to_string()
}

fn current_platform() -> &'static str {
    if cfg!(target_os = "android") {
        "android"
    } else if cfg!(target_os = "ios") {
        "ios"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn parse_moment(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let value = value.trim();
    if let Ok(moment) = chrono::DateTime::parse_from_rfc3339(value) {
        return Some(moment.with_timezone(&chrono::Utc));
    }
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|moment| moment.and_utc())
}

/// An unparseable date hides the announcement so a typo can't broadcast it to everyone.
fn applies(announcement: &Announcement, version: &str, now: chrono::DateTime<chrono::Utc>) -> bool {
    let platform = current_platform();
    if !announcement.platforms.is_empty()
        && !announcement
            .platforms
            .iter()
            .any(|p| p.eq_ignore_ascii_case(platform))
    {
        return false;
    }
    if let Some(min) = &announcement.min_version {
        if is_version_newer(min, version) {
            return false;
        }
    }
    if let Some(max) = &announcement.max_version {
        if is_version_newer(version, max) {
            return false;
        }
    }
    if let Some(start) = &announcement.starts_at {
        match parse_moment(start) {
            Some(start) if now >= start => {}
            _ => return false,
        }
    }
    if let Some(end) = &announcement.ends_at {
        match parse_moment(end) {
            Some(end) if now < end => {}
            _ => return false,
        }
    }
    !announcement.id.trim().is_empty() && !announcement.title.is_empty()
}

#[tauri::command]
pub async fn fetch_app_notices(source: Option<String>) -> Result<Vec<Announcement>, String> {
    let url = match source {
        Some(source) if cfg!(debug_assertions) && !source.trim().is_empty() => source,
        _ => FEED_URL.to_string(),
    };
    let client = crate::net::builder_proxied()
        .timeout(crate::net::defaults().request_timeout)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;
    let response = client.get(&url).send().await.map_err(|e| {
        format!(
            "Failed to fetch announcements: {}",
            crate::net::error_chain(&e)
        )
    })?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    if !response.status().is_success() {
        return Err(format!(
            "Announcements returned status {}",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|len| len > MAX_FEED_BYTES as u64)
    {
        return Err("Announcements file is too large".to_string());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read announcements: {e}"))?;
    if bytes.len() > MAX_FEED_BYTES {
        return Err("Announcements file is too large".to_string());
    }
    let feed: AnnouncementFeed =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid announcements file: {e}"))?;

    let version = env!("CARGO_PKG_VERSION");
    let now = chrono::Utc::now();
    Ok(feed
        .announcements
        .into_iter()
        .filter(|announcement| applies(announcement, version, now))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn announcement(min: Option<&str>, max: Option<&str>) -> Announcement {
        Announcement {
            id: "a".into(),
            severity: "info".into(),
            title: HashMap::from([("en".into(), "t".into())]),
            body: HashMap::new(),
            platforms: Vec::new(),
            min_version: min.map(Into::into),
            max_version: max.map(Into::into),
            starts_at: None,
            ends_at: None,
            actions: Vec::new(),
        }
    }

    #[test]
    fn version_bounds_are_inclusive() {
        let now = chrono::Utc::now();
        assert!(applies(
            &announcement(Some("26.10.1-1"), Some("26.10.1-2")),
            "26.10.1-1",
            now
        ));
        assert!(applies(
            &announcement(None, Some("26.10.1-2")),
            "26.10.1-2",
            now
        ));
        assert!(!applies(
            &announcement(None, Some("26.10.1-1")),
            "26.10.1-2",
            now
        ));
        assert!(!applies(
            &announcement(Some("26.10.2"), None),
            "26.10.1-2",
            now
        ));
    }

    #[test]
    fn unparseable_dates_hide_the_announcement() {
        let now = chrono::Utc::now();
        let mut item = announcement(None, None);
        item.ends_at = Some("soon".into());
        assert!(!applies(&item, "26.10.1", now));
        item.ends_at = Some("2999-01-01".into());
        assert!(applies(&item, "26.10.1", now));
    }
}
