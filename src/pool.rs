//! Core proxy pool implementation.

use crate::config::{HostConfig, ProxySelectionStrategy};
use crate::error::NoProxyAvailable;
use crate::proxy::{Proxy, ProxyStatus};
use crate::utils;

use futures::future;
use log::{info, warn};
use parking_lot::{Mutex, RwLock};
use rand::RngExt;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{self};

/// A pool of proxies that can be used for HTTP requests.
pub struct ProxyPool {
    /// Source URLs to fetch proxy lists from.
    sources: Vec<String>,
    /// All proxies in the pool.
    proxies: RwLock<Vec<Proxy>>,
    /// Host-level configuration for the pool.
    pub config: HostConfig,
    /// Used for round-robin proxy selection.
    last_proxy_index: Mutex<usize>,
}

impl ProxyPool {
    /// Create a new proxy pool with the given configuration.
    /// This will fetch proxies from sources and perform health checks synchronously.
    pub async fn new(
        sources: Vec<String>,
        config: HostConfig,
    ) -> Result<Arc<Self>, reqwest::Error> {
        let pool = Arc::new(Self {
            sources,
            proxies: RwLock::new(Vec::new()),
            config,
            last_proxy_index: Mutex::new(0),
        });

        // Initialize proxies from sources
        pool.initialize_proxies().await?;

        // Perform initial health check synchronously
        info!("Starting synchronous initial health check");
        pool.check_all_proxies().await;

        // Display initial stats
        let (total, healthy) = pool.get_stats();
        info!(
            "Initial proxy pool status: {}/{} healthy proxies",
            healthy, total
        );
        if healthy < pool.config.min_available_proxies {
            warn!(
                "Healthy proxies below minimum for host [{}]: {} < {}",
                pool.config.host(),
                healthy,
                pool.config.min_available_proxies
            );
        }

        // Start background health check task
        let pool_clone = Arc::clone(&pool);
        tokio::spawn(async move {
            loop {
                time::sleep(pool_clone.config.health_check_interval).await;
                pool_clone.check_all_proxies().await;

                let (total, healthy) = pool_clone.get_stats();
                info!(
                    "Proxy pool status update: {}/{} healthy proxies",
                    healthy, total
                );
                if healthy < pool_clone.config.min_available_proxies {
                    warn!(
                        "Healthy proxies below minimum for host [{}]: {} < {}",
                        pool_clone.config.host(),
                        healthy,
                        pool_clone.config.min_available_proxies
                    );
                }
            }
        });

        Ok(pool)
    }

    /// Initialize the proxy pool by fetching proxies from all configured sources.
    async fn initialize_proxies(&self) -> Result<(), reqwest::Error> {
        info!(
            "Initializing proxy pool from {} sources",
            self.sources.len()
        );

        let mut all_proxies = HashSet::new();

        // Fetch proxies from each source
        for source in &self.sources {
            match utils::fetch_proxies_from_source(source).await {
                Ok(source_proxies) => {
                    info!("Fetched {} proxies from {}", source_proxies.len(), source);
                    all_proxies.extend(source_proxies);
                }
                Err(e) => {
                    warn!("Failed to fetch proxies from {}: {}", source, e);
                }
            }
        }

        info!(
            "Found {} unique proxies before health check",
            all_proxies.len()
        );

        // Add proxies to the pool
        {
            let mut proxies = self.proxies.write();
            for url in all_proxies {
                proxies.push(Proxy::new(url, self.config.min_request_interval_ms));
            }
        }

        Ok(())
    }

    /// Check the health of all proxies in the pool.
    pub async fn check_all_proxies(&self) {
        info!("Starting health check for all proxies");

        let proxies = {
            let guard = self.proxies.read();
            guard.clone()
        };
        let mut futures = Vec::new();

        for proxy in &proxies {
            let proxy_url = proxy.url.clone();
            let check_url = self.config.health_check_url.clone();
            let timeout = self.config.health_check_timeout;

            let future = async move {
                let start = Instant::now();

                // Create a client using this proxy
                let reqwest_proxy = match proxy.to_reqwest_proxy() {
                    Ok(proxy) => proxy,
                    Err(_) => return (proxy_url, false, None),
                };

                let proxy_client = match reqwest::Client::builder()
                    .timeout(timeout)
                    .danger_accept_invalid_certs(true)
                    .proxy(reqwest_proxy)
                    .build()
                {
                    Ok(client) => client,
                    Err(_) => return (proxy_url, false, None),
                };

                // Test the proxy.
                match proxy_client.get(&check_url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        let elapsed = start.elapsed().as_secs_f64();
                        (proxy_url, true, Some(elapsed))
                    }
                    _ => (proxy_url, false, None),
                }
            };

            futures.push(future);
        }

        // Run all health checks concurrently
        let results = future::join_all(futures).await;

        let mut healthy_count = 0;
        let mut unhealthy_count = 0;

        // Update proxy statuses based on health check results
        {
            let mut proxies = self.proxies.write();

            for (url, is_healthy, response_time) in results {
                if let Some(proxy) = proxies.iter_mut().find(|p| p.url == url) {
                    let old_status = proxy.status;

                    if is_healthy {
                        proxy.status = ProxyStatus::Healthy;
                        proxy.response_time = response_time;
                        proxy.cooldown_until = None;
                        healthy_count += 1;
                    } else {
                        proxy.status = ProxyStatus::Unhealthy;
                        proxy.cooldown_until = Some(Instant::now() + self.config.proxy_cooldown);
                        unhealthy_count += 1;
                    }

                    // Log status changes
                    if old_status != proxy.status {
                        info!(
                            "Proxy {} status changed: {:?} -> {:?}",
                            proxy.url, old_status, proxy.status
                        );
                    }

                    proxy.last_check = Instant::now();
                }
            }
        }

        info!(
            "Health check completed: {} healthy, {} unhealthy",
            healthy_count, unhealthy_count
        );
    }

    /// Get a proxy from the pool according to the configured selection strategy.
    pub fn get_proxy(&self) -> Result<Proxy, NoProxyAvailable> {
        self.get_proxy_internal(None)
    }

    /// Get a proxy while excluding specific proxy URLs.
    pub fn get_proxy_excluding(
        &self,
        excluded: &HashSet<String>,
    ) -> Result<Proxy, NoProxyAvailable> {
        self.get_proxy_internal(Some(excluded))
    }

    fn get_proxy_internal(
        &self,
        excluded: Option<&HashSet<String>>,
    ) -> Result<Proxy, NoProxyAvailable> {
        let now = Instant::now();
        let mut proxies = self.proxies.write();

        // Move cooled-down proxies into half-open so they can be probed by real traffic.
        for proxy in proxies.iter_mut() {
            if proxy.status == ProxyStatus::Unhealthy
                && proxy
                    .cooldown_until
                    .is_some_and(|cooldown_until| cooldown_until <= now)
            {
                proxy.status = ProxyStatus::HalfOpen;
                proxy.cooldown_until = None;
            }
        }

        // Filter selectable proxies.
        let healthy_proxies: Vec<&Proxy> = proxies
            .iter()
            .filter(|p| matches!(p.status, ProxyStatus::Healthy | ProxyStatus::HalfOpen))
            .filter(|p| excluded.map(|urls| !urls.contains(&p.url)).unwrap_or(true))
            .collect();

        if healthy_proxies.is_empty() {
            return Err(NoProxyAvailable);
        }

        // Select a proxy based on the configured strategy
        let selected = match self.config.selection_strategy {
            ProxySelectionStrategy::FastestResponse => {
                // Select the proxy with the fastest response time
                healthy_proxies
                    .iter()
                    .min_by(|a, b| {
                        a.response_time
                            .unwrap_or(f64::MAX)
                            .partial_cmp(&b.response_time.unwrap_or(f64::MAX))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap()
            }
            ProxySelectionStrategy::MostReliable => {
                // Select the proxy with the highest success rate
                healthy_proxies
                    .iter()
                    .max_by(|a, b| {
                        a.success_rate()
                            .partial_cmp(&b.success_rate())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap()
            }
            ProxySelectionStrategy::TopKReliableRandom => {
                let mut ranked = healthy_proxies;
                ranked.sort_by(|a, b| {
                    b.success_rate()
                        .partial_cmp(&a.success_rate())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let top_k = self.config.reliable_top_k.min(ranked.len()).max(1);
                let mut rng = rand::rng();
                let idx = rng.random_range(0..top_k);
                ranked[idx]
            }
            ProxySelectionStrategy::Random => {
                // Select a random healthy proxy
                let mut rng = rand::rng();
                let idx = rng.random_range(0..healthy_proxies.len());
                healthy_proxies[idx]
            }
            ProxySelectionStrategy::RoundRobin => {
                // Round-robin selection
                let mut last_index = self.last_proxy_index.lock();
                *last_index = (*last_index + 1) % healthy_proxies.len();
                healthy_proxies[*last_index]
            }
        };

        Ok((*selected).clone())
    }

    /// Report a successful request through a proxy.
    pub fn report_proxy_success(&self, url: &str) {
        let mut proxies = self.proxies.write();
        if let Some(proxy) = proxies.iter_mut().find(|p| p.url == url) {
            proxy.success_count += 1;
            proxy.status = ProxyStatus::Healthy;
            proxy.cooldown_until = None;
        }
    }

    /// Report a failed request through a proxy.
    pub fn report_proxy_failure(&self, url: &str) {
        let mut proxies = self.proxies.write();
        if let Some(proxy) = proxies.iter_mut().find(|p| p.url == url) {
            proxy.failure_count += 1;
            let old_status = proxy.status;
            proxy.status = ProxyStatus::Unhealthy;
            proxy.cooldown_until = Some(Instant::now() + self.config.proxy_cooldown);

            if old_status != ProxyStatus::Unhealthy {
                warn!(
                    "Proxy {} marked unhealthy: {} failures, {} successes, cooldown {:?}",
                    proxy.url, proxy.failure_count, proxy.success_count, self.config.proxy_cooldown
                );
            }
        }
    }

    /// Get statistics about the proxy pool.
    pub fn get_stats(&self) -> (usize, usize) {
        let proxies = self.proxies.read();
        let total = proxies.len();
        let healthy = proxies
            .iter()
            .filter(|p| matches!(p.status, ProxyStatus::Healthy | ProxyStatus::HalfOpen))
            .count();

        (total, healthy)
    }
}
