//! Configuration for the proxy pool.

use crate::classifier::{BodyClassifier, DefaultBodyClassifier};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

/// Factory used by middleware to create request clients before attaching proxy.
pub type ClientBuilderFactory = Arc<dyn Fn() -> reqwest::ClientBuilder + Send + Sync>;

/// Strategy for selecting a proxy from the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxySelectionStrategy {
    /// Select the proxy with the fastest response time.
    FastestResponse,
    /// Select the proxy with the highest success rate.
    MostReliable,
    /// Randomly select one proxy from Top-K by success rate.
    TopKReliableRandom,
    /// Select a random healthy proxy.
    Random,
    /// Select proxies in round-robin fashion.
    RoundRobin,
}

/// Retry strategy for request retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    /// Keep current behavior: each retry re-selects by `selection_strategy`
    /// and may pick the same proxy again.
    DefaultSelection,
    /// On retries, always pick a proxy that has not been used by this request yet.
    NewProxyOnRetry,
}

/// Per-host configuration.
///
/// Each `HostConfig` initializes one dedicated proxy pool.
#[derive(Clone)]
pub struct HostConfig {
    pub(crate) host: String,
    pub(crate) primary: bool,
    /// Interval between health checks.
    pub(crate) health_check_interval: Duration,
    /// Timeout for health checks.
    pub(crate) health_check_timeout: Duration,
    /// Minimum number of available proxies.
    pub(crate) min_available_proxies: usize,
    /// URL used for health checks.
    pub(crate) health_check_url: String,
    /// Number of times to retry a request with different proxies.
    pub(crate) retry_count: usize,
    /// Retry behavior.
    pub(crate) retry_strategy: RetryStrategy,
    /// Strategy for selecting proxies.
    pub(crate) selection_strategy: ProxySelectionStrategy,
    /// Minimum interval between requests on the same proxy instance.
    pub(crate) min_request_interval_ms: u64,
    /// Body classifier for business-level proxy health feedback.
    pub(crate) body_classifier: Arc<dyn BodyClassifier>,
    /// Cooldown duration after a proxy failure.
    pub(crate) proxy_cooldown: Duration,
    /// K value for `TopKReliableRandom`.
    pub(crate) reliable_top_k: usize,
}

impl fmt::Debug for HostConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HostConfig")
            .field("host", &self.host)
            .field("primary", &self.primary)
            .field("health_check_interval", &self.health_check_interval)
            .field("health_check_timeout", &self.health_check_timeout)
            .field("min_available_proxies", &self.min_available_proxies)
            .field("health_check_url", &self.health_check_url)
            .field("retry_count", &self.retry_count)
            .field("retry_strategy", &self.retry_strategy)
            .field("selection_strategy", &self.selection_strategy)
            .field("min_request_interval_ms", &self.min_request_interval_ms)
            .field("body_classifier", &"<dyn BodyClassifier>")
            .field("proxy_cooldown", &self.proxy_cooldown)
            .field("reliable_top_k", &self.reliable_top_k)
            .finish()
    }
}

impl HostConfig {
    /// Create a new builder.
    pub fn builder(host: impl Into<String>) -> HostConfigBuilder {
        HostConfigBuilder::new(host)
    }

    /// Bound host.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Whether this host is the primary fallback host pool.
    pub fn primary(&self) -> bool {
        self.primary
    }

    /// Interval between health checks.
    pub fn health_check_interval(&self) -> Duration {
        self.health_check_interval
    }

    /// Timeout for health checks.
    pub fn health_check_timeout(&self) -> Duration {
        self.health_check_timeout
    }

    /// Minimum number of available proxies.
    pub fn min_available_proxies(&self) -> usize {
        self.min_available_proxies
    }

    /// URL used for health checks.
    pub fn health_check_url(&self) -> &str {
        &self.health_check_url
    }

    /// Number of times to retry.
    pub fn retry_count(&self) -> usize {
        self.retry_count
    }

    /// Retry strategy.
    pub fn retry_strategy(&self) -> RetryStrategy {
        self.retry_strategy
    }

    /// Selection strategy.
    pub fn selection_strategy(&self) -> ProxySelectionStrategy {
        self.selection_strategy
    }

    /// Minimum interval between requests on the same proxy instance.
    pub fn min_request_interval_ms(&self) -> u64 {
        self.min_request_interval_ms
    }

    /// Body classifier.
    pub fn body_classifier(&self) -> &Arc<dyn BodyClassifier> {
        &self.body_classifier
    }

    /// Cooldown duration after a proxy failure.
    pub fn proxy_cooldown(&self) -> Duration {
        self.proxy_cooldown
    }

    /// K value for `TopKReliableRandom`.
    pub fn reliable_top_k(&self) -> usize {
        self.reliable_top_k
    }
}

/// Builder for `HostConfig`.
pub struct HostConfigBuilder {
    host: String,
    primary: bool,
    health_check_interval: Option<Duration>,
    health_check_timeout: Option<Duration>,
    min_available_proxies: Option<usize>,
    health_check_url: Option<String>,
    retry_count: Option<usize>,
    retry_strategy: Option<RetryStrategy>,
    selection_strategy: Option<ProxySelectionStrategy>,
    min_request_interval_ms: Option<u64>,
    body_classifier: Option<Arc<dyn BodyClassifier>>,
    proxy_cooldown: Option<Duration>,
    reliable_top_k: Option<usize>,
}

impl HostConfigBuilder {
    /// Create builder with a target host.
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: normalize_host(host.into()),
            primary: false,
            health_check_interval: None,
            health_check_timeout: None,
            min_available_proxies: None,
            health_check_url: None,
            retry_count: None,
            retry_strategy: None,
            selection_strategy: None,
            min_request_interval_ms: None,
            body_classifier: None,
            proxy_cooldown: None,
            reliable_top_k: None,
        }
    }

    /// Set the interval between health checks.
    pub fn health_check_interval(mut self, interval: Duration) -> Self {
        self.health_check_interval = Some(interval);
        self
    }

    /// Set whether this host is primary fallback.
    pub fn primary(mut self, primary: bool) -> Self {
        self.primary = primary;
        self
    }

    /// Set the timeout for health checks.
    pub fn health_check_timeout(mut self, timeout: Duration) -> Self {
        self.health_check_timeout = Some(timeout);
        self
    }

    /// Set the minimum number of available proxies.
    pub fn min_available_proxies(mut self, count: usize) -> Self {
        self.min_available_proxies = Some(count);
        self
    }

    /// Set the URL used for health checks.
    pub fn health_check_url(mut self, url: impl Into<String>) -> Self {
        self.health_check_url = Some(url.into());
        self
    }

    /// Set retry count.
    pub fn retry_count(mut self, count: usize) -> Self {
        self.retry_count = Some(count);
        self
    }

    /// Set retry strategy.
    pub fn retry_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.retry_strategy = Some(strategy);
        self
    }

    /// Set selection strategy.
    pub fn selection_strategy(mut self, strategy: ProxySelectionStrategy) -> Self {
        self.selection_strategy = Some(strategy);
        self
    }

    /// Set minimum interval milliseconds between requests on one proxy instance.
    pub fn min_request_interval_ms(mut self, interval_ms: u64) -> Self {
        self.min_request_interval_ms = Some(interval_ms);
        self
    }

    /// Set custom body classifier.
    pub fn body_classifier(mut self, classifier: impl BodyClassifier) -> Self {
        self.body_classifier = Some(Arc::new(classifier));
        self
    }

    /// Set cooldown duration after one failed request on a proxy.
    pub fn proxy_cooldown(mut self, cooldown: Duration) -> Self {
        self.proxy_cooldown = Some(cooldown);
        self
    }

    /// Set K for `TopKReliableRandom`.
    pub fn reliable_top_k(mut self, top_k: usize) -> Self {
        self.reliable_top_k = Some(top_k.max(1));
        self
    }

    /// Build host config.
    pub fn build(self) -> HostConfig {
        let health_check_url = self
            .health_check_url
            .unwrap_or_else(|| "https://www.google.com".to_string());
        let health_check_url = if health_check_url.trim().is_empty() {
            "https://www.google.com".to_string()
        } else {
            health_check_url
        };

        HostConfig {
            host: if self.host.is_empty() {
                "default".to_string()
            } else {
                self.host
            },
            primary: self.primary,
            health_check_interval: self
                .health_check_interval
                .unwrap_or(Duration::from_secs(300)),
            health_check_timeout: self.health_check_timeout.unwrap_or(Duration::from_secs(10)),
            min_available_proxies: self.min_available_proxies.unwrap_or(3),
            health_check_url,
            retry_count: self.retry_count.unwrap_or(3),
            retry_strategy: self
                .retry_strategy
                .unwrap_or(RetryStrategy::DefaultSelection),
            selection_strategy: self
                .selection_strategy
                .unwrap_or(ProxySelectionStrategy::FastestResponse),
            min_request_interval_ms: self.min_request_interval_ms.unwrap_or(500).max(1),
            body_classifier: self
                .body_classifier
                .unwrap_or_else(|| Arc::new(DefaultBodyClassifier)),
            proxy_cooldown: self.proxy_cooldown.unwrap_or(Duration::from_secs(30)),
            reliable_top_k: self.reliable_top_k.unwrap_or(8).max(1),
        }
    }
}

/// Top-level configuration.
#[derive(Clone)]
pub struct ProxyPoolConfig {
    /// Shared source URLs used to build proxy lists for all host pools.
    pub(crate) sources: Vec<String>,
    /// Host-specific pool definitions.
    pub(crate) hosts: Vec<HostConfig>,
    /// Factory used by middleware to create request clients before attaching proxy.
    pub(crate) client_builder_factory: ClientBuilderFactory,
}

impl fmt::Debug for ProxyPoolConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyPoolConfig")
            .field("sources", &self.sources)
            .field("hosts", &self.hosts)
            .field(
                "client_builder_factory",
                &"<dyn Fn() -> reqwest::ClientBuilder>",
            )
            .finish()
    }
}

impl ProxyPoolConfig {
    /// Create builder.
    pub fn builder() -> ProxyPoolConfigBuilder {
        ProxyPoolConfigBuilder::new()
    }

    /// Sources.
    pub fn sources(&self) -> &[String] {
        &self.sources
    }

    /// Host configs.
    pub fn hosts(&self) -> &[HostConfig] {
        &self.hosts
    }

    /// Factory used by middleware to create request clients before attaching proxy.
    pub fn client_builder_factory(&self) -> &ClientBuilderFactory {
        &self.client_builder_factory
    }
}

/// Builder for `ProxyPoolConfig`.
pub struct ProxyPoolConfigBuilder {
    sources: Vec<String>,
    hosts: Vec<HostConfig>,
    client_builder_factory: Option<ClientBuilderFactory>,
}

impl ProxyPoolConfigBuilder {
    /// Create builder.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            hosts: Vec::new(),
            client_builder_factory: None,
        }
    }

    /// Set source URLs.
    pub fn sources(mut self, sources: Vec<impl Into<String>>) -> Self {
        self.sources = sources.into_iter().map(Into::into).collect();
        self
    }

    /// Set all host configs.
    ///
    /// Exactly one host should set `primary(true)` as fallback for unknown hosts.
    pub fn hosts(mut self, hosts: Vec<HostConfig>) -> Self {
        self.hosts = hosts;
        self
    }

    /// Add one host config.
    ///
    /// Exactly one host should set `primary(true)` as fallback for unknown hosts.
    pub fn add_host(mut self, host: HostConfig) -> Self {
        self.hosts.push(host);
        self
    }

    /// Set request client builder factory.
    ///
    /// Middleware will call this factory on each attempt, then append proxy settings.
    /// Use this to keep timeout/pool/TLS settings aligned with your outer client setup.
    pub fn client_builder_factory<F>(mut self, factory: F) -> Self
    where
        F: Fn() -> reqwest::ClientBuilder + Send + Sync + 'static,
    {
        self.client_builder_factory = Some(Arc::new(factory));
        self
    }

    /// Build config.
    pub fn build(self) -> ProxyPoolConfig {
        ProxyPoolConfig {
            sources: self.sources,
            hosts: self.hosts,
            client_builder_factory: self
                .client_builder_factory
                .unwrap_or_else(|| Arc::new(reqwest::Client::builder)),
        }
    }
}

impl Default for ProxyPoolConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_host(host: String) -> String {
    host.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{HostConfig, ProxyPoolConfig};

    #[test]
    fn host_config_normalizes_host() {
        let host = HostConfig::builder(" API.EXAMPLE.COM ").build();
        assert_eq!(host.host(), "api.example.com");
    }

    #[test]
    fn pool_config_keeps_hosts() {
        let api = HostConfig::builder("api.example.com").build();
        let web = HostConfig::builder("web.example.com").build();
        let config = ProxyPoolConfig::builder().hosts(vec![api, web]).build();
        assert_eq!(config.hosts().len(), 2);
    }
}
