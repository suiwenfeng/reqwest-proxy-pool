//! Proxy representation and status.

use governor::{clock::DefaultClock, middleware::NoOpMiddleware, state::{InMemoryState, NotKeyed}, Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

/// Status of a proxy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProxyStatus {
    /// The proxy has not been tested yet.
    Unknown,
    /// The proxy is healthy and can be used.
    Healthy,
    /// The proxy is unhealthy and should not be used.
    Unhealthy,
}

/// Representation of a proxy server.
#[derive(Debug, Clone)]
pub struct Proxy {
    /// The URL of the proxy (e.g. "socks5://127.0.0.1:1080").
    pub url: String,
    /// The current status of the proxy.
    pub status: ProxyStatus,
    /// Number of successful requests made through this proxy.
    pub success_count: usize,
    /// Number of failed requests made through this proxy.
    pub failure_count: usize,
    /// Time when this proxy was last checked.
    pub last_check: Instant,
    /// Average response time in seconds, if available.
    pub response_time: Option<f64>,
    /// Rate limiter to control requests per second.
    pub limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
}

impl Proxy {
    /// Create a new proxy with the given URL and rate limit.
    pub fn new(url: String, max_rps: f64) -> Self {
        // Create a rate limiter for this proxy
        let quota = Quota::per_second(NonZeroU32::new(max_rps.ceil() as u32).unwrap_or(NonZeroU32::new(1).unwrap()));
        let limiter = Arc::new(RateLimiter::direct(quota));
        
        Self {
            url,
            status: ProxyStatus::Unknown,
            success_count: 0,
            failure_count: 0,
            last_check: Instant::now(),
            response_time: None,
            limiter,
        }
    }
    
    /// Convert the proxy URL to a reqwest::Proxy.
    pub fn to_reqwest_proxy(&self) -> Result<reqwest::Proxy, reqwest::Error> {
        reqwest::Proxy::all(&self.url)
    }
    
    /// Calculate the success rate of this proxy.
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            return 0.0;
        }
        self.success_count as f64 / total as f64
    }
}