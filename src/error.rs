//! Error types for the reqwest-proxy-pool crate.

use thiserror::Error;

/// Error returned when no healthy proxy is available in the pool.
#[derive(Debug, Error)]
#[error("No proxy available in pool")]
pub struct NoProxyAvailable;
