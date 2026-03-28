//! # reqwest-proxy-pool
//!
//! A SOCKS5/SOCKS5H proxy pool middleware for reqwest.
//!
//! This library provides host-based SOCKS5 proxy pools for reqwest middleware:
//! - one `HostConfig` defines one host-specific proxy pool
//! - requests are routed by host
//! - unknown hosts fall back to the unique `primary=true` host pool

pub mod classifier;
pub mod config;
pub mod error;
pub mod middleware;
pub mod pool;
pub mod proxy;
mod utils;

pub use classifier::{BodyClassifier, DefaultBodyClassifier, ProxyBodyVerdict};
pub use config::{
    HostConfig, HostConfigBuilder, ProxyPoolConfig, ProxyPoolConfigBuilder, ProxySelectionStrategy,
    RetryStrategy,
};
pub use error::NoProxyAvailable;
pub use middleware::ProxyPoolMiddleware;
pub use pool::ProxyPool;
pub use proxy::{Proxy, ProxyStatus};
