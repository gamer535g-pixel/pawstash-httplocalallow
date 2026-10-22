pub mod coomer;
pub mod manager;
pub mod onlyhaven;
pub mod pawchive;
pub mod queue;
pub mod traits;

pub use coomer::CoomerProvider;
pub use manager::ProviderManager;
pub use onlyhaven::OnlyHavenProvider;
pub use pawchive::{PawchiveClient, PawchiveProvider};
pub use queue::{ProviderQueueConfig, ProviderRequestQueue};
pub use traits::{derive_subdomain_url, ProviderConfig, ProviderHealth, SourceProvider};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderPostLink {
    pub service: String,
    pub post_id: String,
    pub creator_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCreatorLink {
    pub service: String,
    pub creator_hint: String,
}

pub(crate) fn clean_link_segment(value: &str) -> Option<String> {
    let value = value.trim_matches('/').trim();
    (!value.is_empty()
        && value.len() <= 160
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        }))
    .then(|| value.to_string())
}

pub fn parse_archive_post_url(url: &reqwest::Url) -> Option<ProviderPostLink> {
    OnlyHavenProvider::parse_archive_post_url(url)
        .or_else(|| CoomerProvider::parse_archive_post_url(url))
        .or_else(|| PawchiveProvider::parse_archive_post_url(url))
}

pub fn parse_archive_creator_url(url: &reqwest::Url) -> Option<ProviderCreatorLink> {
    OnlyHavenProvider::parse_archive_creator_url(url)
        .or_else(|| CoomerProvider::parse_archive_creator_url(url))
        .or_else(|| PawchiveProvider::parse_archive_creator_url(url))
}

pub fn default_pawchive_services() -> Vec<String> {
    PawchiveProvider::default_services()
}

pub fn default_onlyhaven_services() -> Vec<String> {
    OnlyHavenProvider::default_services()
}

pub fn default_coomer_services() -> Vec<String> {
    CoomerProvider::default_services()
}

pub fn uses_app_user_agent(url: &str) -> bool {
    PawchiveProvider::uses_app_user_agent(url)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AiDirective {
    None,
    Hide,
    Only,
}

const HIDE_AI_TOKENS: [&str; 5] = ["?hide=(ai)", "hide=(ai)", "?hide=ai", "hide=ai", "ai:hide"];
const ONLY_AI_TOKENS: [&str; 10] = [
    "?only=(ai)",
    "only=(ai)",
    "?only=ai",
    "only=ai",
    "?show=(ai)",
    "show=(ai)",
    "?show=ai",
    "show=ai",
    "ai:only",
    "ai:show",
];

pub(crate) fn split_ai_directive(query: &str) -> (String, AiDirective) {
    let directive = if HIDE_AI_TOKENS.iter().any(|token| query.contains(token)) {
        AiDirective::Hide
    } else if ONLY_AI_TOKENS.iter().any(|token| query.contains(token)) {
        AiDirective::Only
    } else {
        AiDirective::None
    };
    let mut clean = query.to_string();
    for token in HIDE_AI_TOKENS.iter().chain(ONLY_AI_TOKENS.iter()) {
        clean = clean.replace(token, "");
    }
    (clean.trim().to_string(), directive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_directive_is_split_from_the_search_text() {
        assert_eq!(
            split_ai_directive("hide=ai"),
            (String::new(), AiDirective::Hide)
        );
        assert_eq!(
            split_ai_directive("cat hide=ai"),
            ("cat".to_string(), AiDirective::Hide)
        );
        assert_eq!(
            split_ai_directive("dog ?only=(ai)"),
            ("dog".to_string(), AiDirective::Only)
        );
        assert_eq!(
            split_ai_directive("cat"),
            ("cat".to_string(), AiDirective::None)
        );
    }
}
