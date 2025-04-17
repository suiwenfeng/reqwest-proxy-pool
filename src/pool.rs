//! Core proxy pool implementation.

use crate::config::{ProxyPoolConfig, ProxySelectionStrategy};
use crate::error::NoProxyAvailable;
use crate::proxy::{Proxy, ProxyStatus};
use crate::utils;

use futures::future;
use log::{info, warn};
use parking_lot::{Mutex, RwLock};
use rand::Rng;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{self};

/// A pool of proxies that can be used for HTTP requests.
pub struct ProxyPool {
    /// All proxies in the pool.
    proxies: RwLock<Vec<Proxy>>,
    /// Configuration for the pool.
    pub config: ProxyPoolConfig,
    /// Used for round-robin proxy selection.
    last_proxy_index: Mutex<usize>,
}

impl ProxyPool {
    /// Create a new proxy pool with the given configuration.
    /// This will fetch proxies from sources and perform health checks synchronously.
    pub async fn new(config: ProxyPoolConfig) -> Result<Arc<Self>, reqwest::Error> {
        let pool = Arc::new(Self {
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
        info!("Initial proxy pool status: {}/{} healthy proxies", healthy, total);
        
        // Start background health check task
        let pool_clone = Arc::clone(&pool);
        tokio::spawn(async move {
            loop {
                time::sleep(pool_clone.config.health_check_interval).await;
                pool_clone.check_all_proxies().await;
                
                let (total, healthy) = pool_clone.get_stats();
                info!("Proxy pool status update: {}/{} healthy proxies", healthy, total);
            }
        });
        
        Ok(pool)
    }
    
    /// Initialize the proxy pool by fetching proxies from all configured sources.
    async fn initialize_proxies(&self) -> Result<(), reqwest::Error> {
        info!("Initializing proxy pool from {} sources", self.config.sources.len());
        
        let mut all_proxies = HashSet::new();
        
        // Fetch proxies from each source
        for source in &self.config.sources {
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
        
        info!("Found {} unique proxies before health check", all_proxies.len());
        
        // Add proxies to the pool
        {
            let mut proxies = self.proxies.write();
            for url in all_proxies {
                proxies.push(Proxy::new(url, self.config.max_requests_per_second));
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
                let proxy_client = match reqwest::Client::builder()
                    .timeout(timeout)
                    .proxy(reqwest::Proxy::all(&proxy_url).unwrap_or_else(|_| {
                        // 正确指定返回类型为 Option<reqwest::Url>
                        reqwest::Proxy::custom(move |_| -> Option<reqwest::Url> { None })
                    }))
                    .build() {
                    Ok(client) => client,
                    Err(_) => return (proxy_url, false, None),
                };
                
                // Test the proxy
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
                        healthy_count += 1;
                    } else {
                        proxy.status = ProxyStatus::Unhealthy;
                        unhealthy_count += 1;
                    }
                    
                    // Log status changes
                    if old_status != proxy.status {
                        info!("Proxy {} status changed: {:?} -> {:?}", 
                            proxy.url, old_status, proxy.status);
                    }
                    
                    proxy.last_check = Instant::now();
                }
            }
        }
        
        info!("Health check completed: {} healthy, {} unhealthy", 
            healthy_count, unhealthy_count);
    }
    
    /// Get a proxy from the pool according to the configured selection strategy.
    pub fn get_proxy(&self) -> Result<Proxy, NoProxyAvailable> {
        let proxies = self.proxies.read();
        
        // Filter healthy proxies
        let healthy_proxies: Vec<&Proxy> = proxies.iter()
            .filter(|p| p.status == ProxyStatus::Healthy)
            .collect();
            
        if healthy_proxies.is_empty() {
            return Err(NoProxyAvailable);
        }
        
        // Select a proxy based on the configured strategy
        let selected = match self.config.selection_strategy {
            ProxySelectionStrategy::FastestResponse => {
                // Select the proxy with the fastest response time
                healthy_proxies.iter()
                    .min_by(|a, b| {
                        a.response_time.unwrap_or(f64::MAX)
                        .partial_cmp(&b.response_time.unwrap_or(f64::MAX))
                        .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap()
            },
            ProxySelectionStrategy::MostReliable => {
                // Select the proxy with the highest success rate
                healthy_proxies.iter()
                    .max_by(|a, b| {
                        a.success_rate().partial_cmp(&b.success_rate())
                        .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap()
            },
            ProxySelectionStrategy::Random => {
                // Select a random healthy proxy
                let mut rng = rand::rng();
                let idx = rng.random_range(0..healthy_proxies.len());
                &healthy_proxies[idx]
            },
            ProxySelectionStrategy::RoundRobin => {
                // Round-robin selection
                let mut last_index = self.last_proxy_index.lock();
                *last_index = (*last_index + 1) % healthy_proxies.len();
                &healthy_proxies[*last_index]
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
        }
    }
    
    /// Report a failed request through a proxy.
    pub fn report_proxy_failure(&self, url: &str) {
        let mut proxies = self.proxies.write();
        if let Some(proxy) = proxies.iter_mut().find(|p| p.url == url) {
            proxy.failure_count += 1;
            
            // Mark as unhealthy if failure ratio is too high
            let failure_ratio = proxy.failure_count as f64 / 
                (proxy.success_count + proxy.failure_count) as f64;
                
            if failure_ratio > 0.5 && proxy.failure_count >= 3 {
                let old_status = proxy.status;
                proxy.status = ProxyStatus::Unhealthy;
                
                if old_status != ProxyStatus::Unhealthy {
                    warn!("Proxy {} marked unhealthy: {} failures, {} successes", 
                        proxy.url, proxy.failure_count, proxy.success_count);
                }
            }
        }
    }
    
    /// Get statistics about the proxy pool.
    pub fn get_stats(&self) -> (usize, usize) {
        let proxies = self.proxies.read();
        let total = proxies.len();
        let healthy = proxies.iter()
            .filter(|p| p.status == ProxyStatus::Healthy)
            .count();
            
        (total, healthy)
    }
}
