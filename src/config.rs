//! Configuration for the proxy pool.

use crate::classifier::{DefaultResponseClassifier, ResponseClassifier};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

/// Strategy for selecting a proxy from the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxySelectionStrategy {
    /// Select the proxy with the fastest response time.
    FastestResponse,
    /// Select the proxy with the highest success rate.
    MostReliable,
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
    /// Response classifier for business-level proxy health feedback.
    pub(crate) response_classifier: Arc<dyn ResponseClassifier>,
    /// Accept invalid TLS certificates (needed for most free SOCKS5 proxies).
    pub(crate) danger_accept_invalid_certs: bool,
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
            .field("response_classifier", &"<dyn ResponseClassifier>")
            .field(
                "danger_accept_invalid_certs",
                &self.danger_accept_invalid_certs,
            )
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

    /// Response classifier.
    pub fn response_classifier(&self) -> &Arc<dyn ResponseClassifier> {
        &self.response_classifier
    }

    /// Whether invalid TLS certificates are accepted.
    pub fn danger_accept_invalid_certs(&self) -> bool {
        self.danger_accept_invalid_certs
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
    response_classifier: Option<Arc<dyn ResponseClassifier>>,
    danger_accept_invalid_certs: bool,
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
            response_classifier: None,
            danger_accept_invalid_certs: false,
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

    /// Set custom classifier.
    pub fn response_classifier(mut self, classifier: impl ResponseClassifier) -> Self {
        self.response_classifier = Some(Arc::new(classifier));
        self
    }

    /// Accept invalid TLS certificates.
    pub fn danger_accept_invalid_certs(mut self, accept: bool) -> Self {
        self.danger_accept_invalid_certs = accept;
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
            response_classifier: self
                .response_classifier
                .unwrap_or_else(|| Arc::new(DefaultResponseClassifier)),
            danger_accept_invalid_certs: self.danger_accept_invalid_certs,
        }
    }
}

/// Top-level configuration.
#[derive(Clone, Debug)]
pub struct ProxyPoolConfig {
    /// Shared source URLs used to build proxy lists for all host pools.
    pub(crate) sources: Vec<String>,
    /// Host-specific pool definitions.
    pub(crate) hosts: Vec<HostConfig>,
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
}

/// Builder for `ProxyPoolConfig`.
pub struct ProxyPoolConfigBuilder {
    sources: Vec<String>,
    hosts: Vec<HostConfig>,
}

impl ProxyPoolConfigBuilder {
    /// Create builder.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            hosts: Vec::new(),
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

    /// Build config.
    pub fn build(self) -> ProxyPoolConfig {
        ProxyPoolConfig {
            sources: self.sources,
            hosts: self.hosts,
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
