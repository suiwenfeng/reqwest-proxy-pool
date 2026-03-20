//! Configuration for the proxy pool.

use crate::classifier::{DefaultResponseClassifier, ResponseClassifier};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

/// Strategy for selecting a proxy from the pool.
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Configuration for the proxy pool.
#[derive(Clone)]
pub struct ProxyPoolConfig {
    /// Source URLs to fetch proxy lists from.
    pub sources: Vec<String>,
    /// Interval between health checks.
    pub health_check_interval: Duration,
    /// Timeout for health checks.
    pub health_check_timeout: Duration,
    /// Minimum number of available proxies.
    pub min_available_proxies: usize,
    /// URL used for health checks.
    pub health_check_url: String,
    /// Number of times to retry a request with different proxies.
    pub retry_count: usize,
    /// Strategy for selecting proxies.
    pub selection_strategy: ProxySelectionStrategy,
    /// Maximum requests per second per proxy.
    pub max_requests_per_second: f64,
    /// Response classifier for business-level proxy health feedback.
    pub response_classifier: Arc<dyn ResponseClassifier>,
    /// Accept invalid TLS certificates (needed for most free SOCKS5 proxies).
    pub danger_accept_invalid_certs: bool,
}

impl fmt::Debug for ProxyPoolConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyPoolConfig")
            .field("sources", &self.sources)
            .field("health_check_interval", &self.health_check_interval)
            .field("health_check_timeout", &self.health_check_timeout)
            .field("min_available_proxies", &self.min_available_proxies)
            .field("health_check_url", &self.health_check_url)
            .field("retry_count", &self.retry_count)
            .field("selection_strategy", &self.selection_strategy)
            .field("max_requests_per_second", &self.max_requests_per_second)
            .field("response_classifier", &"<dyn ResponseClassifier>")
            .field(
                "danger_accept_invalid_certs",
                &self.danger_accept_invalid_certs,
            )
            .finish()
    }
}

impl ProxyPoolConfig {
    /// Create a new configuration builder.
    pub fn builder() -> ProxyPoolConfigBuilder {
        ProxyPoolConfigBuilder::new()
    }
}

/// Builder for `ProxyPoolConfig`.
pub struct ProxyPoolConfigBuilder {
    sources: Vec<String>,
    health_check_interval: Option<Duration>,
    health_check_timeout: Option<Duration>,
    min_available_proxies: Option<usize>,
    health_check_url: Option<String>,
    retry_count: Option<usize>,
    selection_strategy: Option<ProxySelectionStrategy>,
    max_requests_per_second: Option<f64>,
    response_classifier: Option<Arc<dyn ResponseClassifier>>,
    danger_accept_invalid_certs: bool,
}

impl ProxyPoolConfigBuilder {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            health_check_interval: None,
            health_check_timeout: None,
            min_available_proxies: None,
            health_check_url: None,
            retry_count: None,
            selection_strategy: None,
            max_requests_per_second: None,
            response_classifier: None,
            danger_accept_invalid_certs: false,
        }
    }

    /// Set the source URLs to fetch proxy lists from.
    pub fn sources(mut self, sources: Vec<impl Into<String>>) -> Self {
        self.sources = sources.into_iter().map(Into::into).collect();
        self
    }

    /// Set the interval between health checks.
    pub fn health_check_interval(mut self, interval: Duration) -> Self {
        self.health_check_interval = Some(interval);
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

    /// Set the number of times to retry a request with different proxies.
    pub fn retry_count(mut self, count: usize) -> Self {
        self.retry_count = Some(count);
        self
    }

    /// Set the strategy for selecting proxies.
    pub fn selection_strategy(mut self, strategy: ProxySelectionStrategy) -> Self {
        self.selection_strategy = Some(strategy);
        self
    }

    /// Set the maximum requests per second per proxy.
    pub fn max_requests_per_second(mut self, rps: f64) -> Self {
        self.max_requests_per_second = Some(rps);
        self
    }

    /// Set a custom response classifier for business-level proxy health feedback.
    ///
    /// Use this to detect captchas, anti-bot pages, or other blocking responses
    /// that pass HTTP-level checks but indicate the proxy is unusable.
    pub fn response_classifier(mut self, classifier: impl ResponseClassifier) -> Self {
        self.response_classifier = Some(Arc::new(classifier));
        self
    }

    /// Accept invalid TLS certificates when connecting through proxies.
    /// Required for most free SOCKS5 proxies that perform TLS interception.
    /// Default: `false`.
    pub fn danger_accept_invalid_certs(mut self, accept: bool) -> Self {
        self.danger_accept_invalid_certs = accept;
        self
    }

    /// Build the configuration.
    pub fn build(self) -> ProxyPoolConfig {
        ProxyPoolConfig {
            sources: self.sources,
            health_check_interval: self
                .health_check_interval
                .unwrap_or(Duration::from_secs(300)),
            health_check_timeout: self.health_check_timeout.unwrap_or(Duration::from_secs(10)),
            min_available_proxies: self.min_available_proxies.unwrap_or(3),
            health_check_url: self
                .health_check_url
                .unwrap_or_else(|| "https://www.google.com".to_string()),
            retry_count: self.retry_count.unwrap_or(3),
            selection_strategy: self
                .selection_strategy
                .unwrap_or(ProxySelectionStrategy::FastestResponse),
            max_requests_per_second: self.max_requests_per_second.unwrap_or(5.0),
            response_classifier: self
                .response_classifier
                .unwrap_or_else(|| Arc::new(DefaultResponseClassifier)),
            danger_accept_invalid_certs: self.danger_accept_invalid_certs,
        }
    }
}

impl Default for ProxyPoolConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
