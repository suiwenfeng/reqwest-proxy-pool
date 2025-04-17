//! Middleware implementation for reqwest.

use crate::config::ProxyPoolConfig;
use crate::error::NoProxyAvailable;
use crate::pool::ProxyPool;

use anyhow::anyhow;
use async_trait::async_trait;
use log::{info, warn};
use reqwest_middleware::{Error, Middleware, Next, Result};
use std::sync::Arc;

/// Middleware that uses a pool of proxies for HTTP requests.
#[derive(Clone)]
pub struct ProxyPoolMiddleware {
    /// The proxy pool.
    pool: Arc<ProxyPool>,
}

impl ProxyPoolMiddleware {
    /// Create a new proxy pool middleware with the given configuration.
    /// This will synchronously initialize the proxy pool and perform health checks.
    pub async fn new(config: ProxyPoolConfig) -> Result<Self> {
        match ProxyPool::new(config).await {
            Ok(pool) => {
                let (total, healthy) = pool.get_stats();
                info!("Proxy pool initialized with {}/{} healthy proxies", healthy, total);
                
                if healthy == 0 {
                    warn!("No healthy proxies available in pool");
                }
                
                Ok(Self { pool })
            }
            Err(e) => {
                Err(Error::Reqwest(e))
            }
        }
    }
}

#[async_trait]
impl Middleware for ProxyPoolMiddleware {
    async fn handle(
        &self,
        req: reqwest::Request,
        _extensions: &mut http::Extensions,
        _next: Next<'_>,
    ) -> Result<reqwest::Response> {
        let max_retries = self.pool.config.retry_count;
        let mut retry_count = 0;
        
        loop {
            // Try to get a healthy proxy
            match self.pool.get_proxy() {
                Ok(proxy) => {
                    let proxied_request = req.try_clone().ok_or_else(|| {
                        Error::Middleware(anyhow!(
                            "Request object is not cloneable. Are you passing a streaming body?"
                                .to_string()
                        ))
                    })?;

                    let proxy_url = proxy.url.clone();
                    info!("Using proxy: {} (attempt {})", proxy_url, retry_count + 1);
                    
                    // Apply rate limiting
                    proxy.limiter.until_ready().await;
                    
                    // Create a new client with the selected proxy
                    let reqwest_proxy = match proxy.to_reqwest_proxy() {
                        Ok(p) => p,
                        Err(e) => {
                            warn!("Failed to create proxy from {}: {}", proxy_url, e);
                            self.pool.report_proxy_failure(&proxy_url);
                            
                            // Try another proxy if available
                            retry_count += 1;
                            if retry_count > max_retries {
                                return Err(Error::Reqwest(e));
                            }
                            continue;
                        }
                    };
                    
                    // Build a new client with the proxy
                    let client = match reqwest::Client::builder()
                        .proxy(reqwest_proxy)
                        .timeout(self.pool.config.health_check_timeout)
                        .build() {
                        Ok(c) => c,
                        Err(e) => {
                            warn!("Failed to build client with proxy {}: {}", proxy_url, e);
                            self.pool.report_proxy_failure(&proxy_url);
                            retry_count += 1;
                            if retry_count > max_retries {
                                return Err(Error::Reqwest(e));
                            }
                            continue;
                        }
                    };
                    
                    // Execute the request and pass extensions
                    match client.execute(proxied_request).await {
                        Ok(response) => {
                            // Request succeeded
                            self.pool.report_proxy_success(&proxy_url);
                            return Ok(response);
                        }
                        Err(err) => {
                            // Request failed
                            warn!("Request failed with proxy {} (attempt {}): {}", 
                                proxy_url, retry_count + 1, err);
                            self.pool.report_proxy_failure(&proxy_url);
                            
                            retry_count += 1;
                            if retry_count > max_retries {
                                return Err(Error::Reqwest(err));
                            }
                            // Loop will continue to try another proxy
                        }
                    }
                }
                Err(_) => {
                    // No healthy proxies available
                    let (total, healthy) = self.pool.get_stats();
                    warn!("No proxy available. Total: {}, Healthy: {}", total, healthy);
                    return Err(Error::Middleware(anyhow!(NoProxyAvailable)));
                }
            }
        }
    }
}
