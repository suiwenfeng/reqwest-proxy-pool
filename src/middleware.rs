//! Middleware implementation for reqwest.

use crate::classifier::ProxyResponseVerdict;
use crate::config::{HostConfig, ProxyPoolConfig, RetryStrategy};
use crate::error::NoProxyAvailable;
use crate::pool::ProxyPool;

use anyhow::anyhow;
use async_trait::async_trait;
use log::{info, warn};
use reqwest_middleware::{Error, Middleware, Next, Result};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Middleware that routes requests to host-bound proxy pools.
#[derive(Clone)]
pub struct ProxyPoolMiddleware {
    /// Host -> pool mapping.
    pools: HashMap<String, Arc<ProxyPool>>,
    /// Primary pool used for unknown hosts.
    primary_host: String,
}

impl ProxyPoolMiddleware {
    /// Create middleware with host-based routing.
    pub async fn new(config: ProxyPoolConfig) -> Result<Self> {
        if config.hosts().is_empty() {
            return Err(Error::Middleware(anyhow!(
                "ProxyPoolConfig must contain at least one HostConfig"
            )));
        }
        if config.sources().is_empty() {
            return Err(Error::Middleware(anyhow!(
                "ProxyPoolConfig.sources cannot be empty"
            )));
        }

        let primary_host = validate_hosts(config.hosts())?;

        let mut pools = HashMap::new();
        for host_config in config.hosts().iter().cloned() {
            let host = host_config.host().to_ascii_lowercase();
            let pool = ProxyPool::new(config.sources().to_vec(), host_config)
                .await
                .map_err(Error::Reqwest)?;
            let (total, healthy) = pool.get_stats();
            info!(
                "Host pool [{}] initialized with {}/{} healthy proxies",
                host, healthy, total
            );
            if healthy == 0 {
                warn!("No healthy proxies available in host pool [{}]", host);
            }

            pools.insert(host, pool);
        }

        Ok(Self {
            pools,
            primary_host,
        })
    }

    fn resolve_pool(&self, req: &reqwest::Request) -> Option<Arc<ProxyPool>> {
        let host = req.url().host_str().map(|h| h.to_ascii_lowercase());
        if let Some(host) = host {
            if let Some(pool) = self.pools.get(&host) {
                return Some(Arc::clone(pool));
            }
        }
        self.pools.get(&self.primary_host).map(Arc::clone)
    }
}

fn validate_hosts(hosts: &[HostConfig]) -> Result<String> {
    let mut seen = HashSet::new();
    let mut primary_hosts = Vec::new();

    for host_config in hosts {
        let host = host_config.host().trim().to_ascii_lowercase();
        if host.is_empty() {
            return Err(Error::Middleware(anyhow!(
                "HostConfig.host cannot be empty"
            )));
        }
        if !seen.insert(host.clone()) {
            return Err(Error::Middleware(anyhow!(
                "duplicate HostConfig for host: {}",
                host
            )));
        }
        if host_config.primary() {
            primary_hosts.push(host);
        }
    }

    if primary_hosts.is_empty() {
        return Err(Error::Middleware(anyhow!(
            "exactly one HostConfig must set primary=true, found none"
        )));
    }
    if primary_hosts.len() > 1 {
        return Err(Error::Middleware(anyhow!(
            "exactly one HostConfig must set primary=true, found {} ({:?})",
            primary_hosts.len(),
            primary_hosts
        )));
    }

    Ok(primary_hosts.remove(0))
}

#[cfg(test)]
mod tests {
    use super::validate_hosts;
    use crate::config::HostConfig;

    #[test]
    fn validate_hosts_requires_one_primary() {
        let hosts = vec![
            HostConfig::builder("a.example.com").build(),
            HostConfig::builder("b.example.com").build(),
        ];
        assert!(validate_hosts(&hosts).is_err());
    }

    #[test]
    fn validate_hosts_rejects_multiple_primary() {
        let hosts = vec![
            HostConfig::builder("a.example.com").primary(true).build(),
            HostConfig::builder("b.example.com").primary(true).build(),
        ];
        assert!(validate_hosts(&hosts).is_err());
    }

    #[test]
    fn validate_hosts_returns_primary_host() {
        let hosts = vec![
            HostConfig::builder("a.example.com").build(),
            HostConfig::builder("b.example.com").primary(true).build(),
        ];
        let primary = validate_hosts(&hosts).expect("primary host should be valid");
        assert_eq!(primary, "b.example.com");
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
        let pool = self.resolve_pool(&req).ok_or_else(|| {
            Error::Middleware(anyhow!(
                "No pool available for request host and no primary host pool configured"
            ))
        })?;

        let max_retries = pool.config.retry_count;
        let mut retry_count = 0;
        let mut used_proxy_urls = HashSet::new();

        loop {
            let proxy_result = match pool.config.retry_strategy {
                RetryStrategy::DefaultSelection => pool.get_proxy(),
                RetryStrategy::NewProxyOnRetry => {
                    if retry_count == 0 {
                        pool.get_proxy()
                    } else {
                        pool.get_proxy_excluding(&used_proxy_urls)
                    }
                }
            };

            match proxy_result {
                Ok(proxy) => {
                    let proxied_request = req.try_clone().ok_or_else(|| {
                        Error::Middleware(anyhow!(
                            "Request object is not cloneable. Are you passing a streaming body?"
                                .to_string()
                        ))
                    })?;

                    let proxy_url = proxy.url.clone();
                    used_proxy_urls.insert(proxy_url.clone());
                    info!("Using proxy: {} (attempt {})", proxy_url, retry_count + 1);

                    proxy.limiter.until_ready().await;

                    let reqwest_proxy = match proxy.to_reqwest_proxy() {
                        Ok(p) => p,
                        Err(e) => {
                            warn!("Failed to create proxy from {}: {}", proxy_url, e);
                            pool.report_proxy_failure(&proxy_url);
                            retry_count += 1;
                            if retry_count > max_retries {
                                return Err(Error::Reqwest(e));
                            }
                            continue;
                        }
                    };

                    let client = match reqwest::Client::builder()
                        .proxy(reqwest_proxy)
                        .timeout(pool.config.health_check_timeout)
                        .danger_accept_invalid_certs(pool.config.danger_accept_invalid_certs)
                        .build()
                    {
                        Ok(c) => c,
                        Err(e) => {
                            warn!("Failed to build client with proxy {}: {}", proxy_url, e);
                            pool.report_proxy_failure(&proxy_url);
                            retry_count += 1;
                            if retry_count > max_retries {
                                return Err(Error::Reqwest(e));
                            }
                            continue;
                        }
                    };

                    match client.execute(proxied_request).await {
                        Ok(response) => match pool.config.response_classifier.classify(&response) {
                            ProxyResponseVerdict::Success => {
                                pool.report_proxy_success(&proxy_url);
                                return Ok(response);
                            }
                            ProxyResponseVerdict::ProxyBlocked => {
                                warn!(
                                    "Proxy {} blocked by target site (attempt {})",
                                    proxy_url,
                                    retry_count + 1
                                );
                                pool.report_proxy_failure(&proxy_url);
                                retry_count += 1;
                                if retry_count > max_retries {
                                    return Ok(response);
                                }
                            }
                            ProxyResponseVerdict::Passthrough => {
                                return Ok(response);
                            }
                        },
                        Err(err) => {
                            warn!(
                                "Request failed with proxy {} (attempt {}): {}",
                                proxy_url,
                                retry_count + 1,
                                err
                            );
                            pool.report_proxy_failure(&proxy_url);
                            retry_count += 1;
                            if retry_count > max_retries {
                                return Err(Error::Reqwest(err));
                            }
                        }
                    }
                }
                Err(_) => {
                    let (total, healthy) = pool.get_stats();
                    warn!(
                        "No proxy available in selected host pool. Total: {}, Healthy: {}",
                        total, healthy
                    );
                    return Err(Error::Middleware(anyhow!(NoProxyAvailable)));
                }
            }
        }
    }
}
