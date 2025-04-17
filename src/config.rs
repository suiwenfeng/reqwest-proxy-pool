//! Configuration for the proxy pool.

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
#[derive(Debug, Clone)]
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

    /// Build the configuration.
    pub fn build(self) -> ProxyPoolConfig {
        ProxyPoolConfig {
            sources: self.sources,
            health_check_interval: self.health_check_interval.unwrap_or(Duration::from_secs(300)),
            health_check_timeout: self.health_check_timeout.unwrap_or(Duration::from_secs(10)),
            min_available_proxies: self.min_available_proxies.unwrap_or(3),
            health_check_url: self.health_check_url.unwrap_or_else(|| "https://www.google.com".to_string()),
            retry_count: self.retry_count.unwrap_or(3),
            selection_strategy: self.selection_strategy.unwrap_or(ProxySelectionStrategy::FastestResponse),
            max_requests_per_second: self.max_requests_per_second.unwrap_or(5.0),
        }
    }
}

impl Default for ProxyPoolConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
