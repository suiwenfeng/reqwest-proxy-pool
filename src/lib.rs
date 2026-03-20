//! # reqwest-proxy-pool
//!
//! A SOCKS5/SOCKS5H proxy pool middleware for reqwest.
//!
//! This library provides a middleware for reqwest that automatically manages a pool of
//! SOCKS5 proxies, testing their health, and using them for requests with automatic retries.

pub mod classifier;
pub mod config;
pub mod error;
pub mod middleware;
pub mod pool;
pub mod proxy;
mod utils;

pub use classifier::{DefaultResponseClassifier, ProxyResponseVerdict, ResponseClassifier};
pub use config::{ProxyPoolConfig, ProxyPoolConfigBuilder, ProxySelectionStrategy};
pub use error::NoProxyAvailable;
pub use middleware::ProxyPoolMiddleware;
pub use pool::ProxyPool;
pub use proxy::{Proxy, ProxyStatus};
