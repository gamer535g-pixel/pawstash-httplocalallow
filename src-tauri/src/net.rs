use crate::config::settings::{AppSettings, ProxyMode};
use std::sync::RwLock;
use std::time::Duration;

pub const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
pub const POOL_MAX_IDLE_PER_HOST: usize = 16;

/// The archive APIs expect Pawstash to identify itself.
pub const DEFAULT_API_USER_AGENT: &str =
    concat!("Github:Pawstash/Pawstash;v=", env!("CARGO_PKG_VERSION"));

/// File hosts and scrapers answer differently to anything that isn't a browser.
pub const DEFAULT_BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 45;
pub const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 8;
pub const DEFAULT_PROVIDER_DEADLINE_SECS: u64 = 8;

#[derive(Debug, Clone)]
pub struct NetworkDefaults {
    pub api_user_agent: String,
    pub browser_user_agent: String,
    pub request_timeout: Duration,
    pub connect_timeout: Duration,
    pub provider_deadline: Duration,
    pub max_provider_timeout: Duration,
    pub proxy: ProxySettings,
    pub host_routes: Vec<HostRoute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRoute {
    pub domain: String,
    pub proxy_url: String,
}

impl HostRoute {
    fn matches(&self, host: &str) -> bool {
        host == self.domain
            || host
                .strip_suffix(self.domain.as_str())
                .is_some_and(|prefix| prefix.ends_with('.'))
    }
}

impl NetworkDefaults {
    pub fn override_for_host(&self, host: &str) -> Option<&HostRoute> {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        self.host_routes
            .iter()
            .filter(|route| route.matches(&host))
            .max_by_key(|route| route.domain.len())
    }
}

fn provider_domains(config: &crate::api::providers::traits::ProviderConfig) -> Vec<String> {
    let mut urls: Vec<&str> = vec![config.api_url.as_str()];
    urls.extend(config.fallback_urls.iter().map(String::as_str));
    urls.extend(config.file_url.as_deref());
    urls.extend(config.image_url.as_deref());

    let mut domains: Vec<String> = urls
        .into_iter()
        .filter_map(|raw| reqwest::Url::parse(raw.trim()).ok())
        .filter_map(|url| url.host_str().map(str::to_ascii_lowercase))
        .map(|host| {
            host.strip_prefix("www.")
                .map(str::to_string)
                .unwrap_or(host)
        })
        .collect();
    domains.sort();
    domains.dedup();
    domains
}

pub(crate) fn host_routes(settings: &AppSettings) -> Vec<HostRoute> {
    let mut routes = Vec::new();
    for provider in settings.providers.iter().filter(|p| p.enabled) {
        if let Some(proxy_url) = provider.effective_proxy_url() {
            routes.extend(
                provider_domains(provider)
                    .into_iter()
                    .map(|domain| HostRoute {
                        domain,
                        proxy_url: proxy_url.clone(),
                    }),
            );
        }
    }
    let cloud_proxy = settings.cloud_proxy_url.trim();
    if !cloud_proxy.is_empty() {
        routes.extend(crate::cloud::NETWORK_HOSTS.iter().map(|domain| HostRoute {
            domain: domain.to_string(),
            proxy_url: cloud_proxy.to_string(),
        }));
    }
    routes
}

#[derive(Debug, Clone, Default)]
pub struct ProxySettings {
    pub mode: ProxyMode,
    pub url: String,
    pub username: String,
    pub password: String,
    pub bypass_local: bool,
}

impl ProxySettings {
    pub fn is_socks(&self) -> bool {
        self.mode == ProxyMode::Custom
            && (self.url.starts_with("socks5://") || self.url.starts_with("socks5h://"))
    }
}

pub fn proxy_for_url(settings: &AppSettings, url: &str) -> ProxySettings {
    let route = reqwest::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .and_then(|host| defaults().override_for_host(&host).cloned());
    match route {
        Some(route) => ProxySettings {
            mode: ProxyMode::Custom,
            url: route.proxy_url,
            username: String::new(),
            password: String::new(),
            bypass_local: true,
        },
        None => ProxySettings::from(settings),
    }
}

impl Default for NetworkDefaults {
    fn default() -> Self {
        Self {
            api_user_agent: DEFAULT_API_USER_AGENT.to_string(),
            browser_user_agent: DEFAULT_BROWSER_USER_AGENT.to_string(),
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS),
            connect_timeout: Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS),
            provider_deadline: Duration::from_secs(DEFAULT_PROVIDER_DEADLINE_SECS),
            max_provider_timeout: Duration::ZERO,
            proxy: ProxySettings::default(),
            host_routes: Vec::new(),
        }
    }
}

static DEFAULTS: RwLock<Option<NetworkDefaults>> = RwLock::new(None);

fn resolved(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn resolved_secs(value: u64, fallback: u64) -> Duration {
    Duration::from_secs(if value == 0 { fallback } else { value })
}

/// Pure, so tests don't touch the process-wide snapshot.
fn resolve(settings: &AppSettings) -> NetworkDefaults {
    NetworkDefaults {
        api_user_agent: resolved(&settings.network_api_user_agent, DEFAULT_API_USER_AGENT),
        browser_user_agent: resolved(
            &settings.network_browser_user_agent,
            DEFAULT_BROWSER_USER_AGENT,
        ),
        request_timeout: resolved_secs(settings.network_timeout_secs, DEFAULT_REQUEST_TIMEOUT_SECS),
        connect_timeout: resolved_secs(
            settings.network_connect_timeout_secs,
            DEFAULT_CONNECT_TIMEOUT_SECS,
        ),
        provider_deadline: resolved_secs(
            settings.provider_deadline_secs,
            DEFAULT_PROVIDER_DEADLINE_SECS,
        ),
        max_provider_timeout: settings
            .providers
            .iter()
            .filter(|p| p.enabled && p.advanced_network && p.timeout_secs > 0)
            .map(|p| Duration::from_secs(p.timeout_secs))
            .max()
            .unwrap_or(Duration::ZERO),
        proxy: ProxySettings::from(settings),
        host_routes: host_routes(settings),
    }
}

pub fn apply_settings(settings: &AppSettings) {
    if let Ok(mut guard) = DEFAULTS.write() {
        *guard = Some(resolve(settings));
    }
    reset_shared_clients();
}

pub fn defaults() -> NetworkDefaults {
    DEFAULTS
        .read()
        .ok()
        .and_then(|guard| guard.clone())
        .unwrap_or_default()
}

pub fn api_user_agent() -> String {
    defaults().api_user_agent
}

pub fn browser_user_agent() -> String {
    defaults().browser_user_agent
}

pub fn provider_deadline() -> Duration {
    defaults().provider_deadline
}

/// The error and every cause under it: reqwest's own message is only "error sending request".
pub fn error_chain(error: &dyn std::error::Error) -> String {
    let mut details = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        details.push_str(&format!(" -> {cause}"));
        source = cause.source();
    }
    details
}

pub fn builder() -> reqwest::ClientBuilder {
    let defaults = defaults();
    reqwest::Client::builder()
        .connect_timeout(defaults.connect_timeout)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
        .tcp_keepalive(Duration::from_secs(60))
}

impl From<&AppSettings> for ProxySettings {
    fn from(settings: &AppSettings) -> Self {
        Self {
            mode: settings.proxy_mode,
            url: settings.proxy_url.clone(),
            username: settings.proxy_username.clone(),
            password: settings.proxy_password.clone(),
            bypass_local: settings.proxy_bypass_local,
        }
    }
}

fn configure_proxy(
    mut builder: reqwest::ClientBuilder,
    proxy: &ProxySettings,
) -> Result<reqwest::ClientBuilder, String> {
    match proxy.mode {
        ProxyMode::None => builder = builder.no_proxy(),
        ProxyMode::System => {}
        ProxyMode::Custom => {
            let url = proxy.url.trim();
            if url.is_empty() {
                return Ok(builder.no_proxy());
            }
            let mut configured =
                reqwest::Proxy::all(url).map_err(|e| format!("Invalid proxy URL: {e}"))?;
            if !proxy.username.is_empty() {
                configured = configured.basic_auth(&proxy.username, &proxy.password);
            }
            if proxy.bypass_local {
                configured =
                    configured.no_proxy(reqwest::NoProxy::from_string("localhost,127.0.0.1,::1"));
            }
            builder = builder.proxy(configured);
        }
    }
    Ok(builder)
}

pub fn apply_proxy(
    builder: reqwest::ClientBuilder,
    settings: &AppSettings,
) -> Result<reqwest::ClientBuilder, String> {
    configure_proxy(builder, &ProxySettings::from(settings))
}

pub fn builder_proxied() -> reqwest::ClientBuilder {
    configure_proxy(builder(), &defaults().proxy).unwrap_or_else(|error| {
        tracing::warn!(%error, "Published proxy failed to apply; connecting directly");
        builder().no_proxy()
    })
}

/// Only schemes the WebView proxy also speaks, so media can't bypass a proxy that API calls use.
pub fn validate_proxy_url(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Ok(());
    }
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid proxy URL '{url}': {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https" | "socks5" | "socks5h") {
        return Err(format!(
            "Unsupported proxy scheme '{}' in '{url}': use http, https, socks5 or socks5h",
            parsed.scheme()
        ));
    }
    reqwest::Proxy::all(url)
        .map(|_| ())
        .map_err(|e| format!("Invalid proxy URL '{url}': {e}"))
}

pub fn builder_with_proxy(settings: &AppSettings) -> Result<reqwest::ClientBuilder, String> {
    apply_proxy(builder(), settings)
}

/// Anything a built client bakes in belongs here, or cached clients go stale.
pub fn client_fingerprint(settings: &AppSettings) -> String {
    let defaults = defaults();
    format!(
        "{:?}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{:?}\u{1}{:?}\u{1}{}\u{1}{}\u{1}{}",
        settings.proxy_mode,
        settings.proxy_url.trim(),
        settings.proxy_username,
        settings.proxy_password,
        settings.proxy_bypass_local,
        defaults.api_user_agent,
        defaults.browser_user_agent,
        defaults.request_timeout,
        defaults.connect_timeout,
        settings.cloud_proxy_url.trim(),
        settings.cloud_user_agent.trim(),
        settings.cloud_max_redirects
    )
}

static SHARED_CLIENT: std::sync::OnceLock<std::sync::Mutex<Option<(String, reqwest::Client)>>> =
    std::sync::OnceLock::new();

pub fn shared_client(settings: &AppSettings) -> Result<reqwest::Client, String> {
    let fingerprint = client_fingerprint(settings);
    let cell = SHARED_CLIENT.get_or_init(|| std::sync::Mutex::new(None));

    if let Ok(guard) = cell.lock() {
        if let Some((cached, client)) = guard.as_ref() {
            if cached == &fingerprint {
                return Ok(client.clone());
            }
        }
    }

    let client = builder_with_proxy(settings)?
        .timeout(defaults().request_timeout)
        .build()
        .map_err(|e| e.to_string())?;

    if let Ok(mut guard) = cell.lock() {
        *guard = Some((fingerprint, client.clone()));
    }
    Ok(client)
}

pub fn reset_shared_clients() {
    if let Some(cell) = SHARED_CLIENT.get() {
        if let Ok(mut guard) = cell.lock() {
            *guard = None;
        }
    }
    crate::downloader::native::NativeDownloader::reset_client_cache();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom_proxy(url: &str) -> AppSettings {
        AppSettings {
            proxy_mode: ProxyMode::Custom,
            proxy_url: url.to_string(),
            ..AppSettings::default()
        }
    }

    #[test]
    fn settings_resolve_to_defaults_or_overrides() {
        let blank = resolve(&AppSettings::default());
        assert_eq!(blank.api_user_agent, DEFAULT_API_USER_AGENT);
        assert_eq!(blank.browser_user_agent, DEFAULT_BROWSER_USER_AGENT);
        assert_eq!(
            blank.request_timeout,
            Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS)
        );
        assert_eq!(
            blank.provider_deadline,
            Duration::from_secs(DEFAULT_PROVIDER_DEADLINE_SECS)
        );

        let overridden = resolve(&AppSettings {
            network_api_user_agent: "  CustomAgent/2.0  ".to_string(),
            network_timeout_secs: 12,
            provider_deadline_secs: 3,
            ..AppSettings::default()
        });
        assert_eq!(overridden.api_user_agent, "CustomAgent/2.0");
        assert_eq!(overridden.request_timeout, Duration::from_secs(12));
        assert_eq!(overridden.provider_deadline, Duration::from_secs(3));
        assert_eq!(overridden.browser_user_agent, DEFAULT_BROWSER_USER_AGENT);
    }

    #[test]
    fn override_proxy_urls_are_validated() {
        for (url, valid) in [
            ("", true),
            ("   ", true),
            ("http://127.0.0.1:8080", true),
            ("https://secure.proxy:443", true),
            ("socks5://user:pass@host:1080", true),
            ("not a url", false),
            ("socks4://host:1080", false),
        ] {
            assert_eq!(validate_proxy_url(url).is_ok(), valid, "{url:?}");
        }
    }

    #[test]
    fn proxy_modes_build_or_report_malformed_urls() {
        let direct = AppSettings {
            proxy_mode: ProxyMode::None,
            ..AppSettings::default()
        };
        assert!(builder_with_proxy(&direct).is_ok());
        assert!(builder_with_proxy(&custom_proxy("   ")).is_ok());
        assert!(builder_with_proxy(&custom_proxy("not a url")).is_err());
    }

    #[test]
    fn client_fingerprint_tracks_proxy_settings() {
        let proxied = AppSettings {
            proxy_username: "user".to_string(),
            ..custom_proxy("http://127.0.0.1:8080")
        };
        assert_eq!(
            client_fingerprint(&proxied),
            client_fingerprint(&proxied.clone())
        );
        assert_ne!(
            client_fingerprint(&proxied),
            client_fingerprint(&custom_proxy(""))
        );
        assert_ne!(
            client_fingerprint(&proxied),
            client_fingerprint(&custom_proxy("http://127.0.0.1:8080"))
        );
    }
}
