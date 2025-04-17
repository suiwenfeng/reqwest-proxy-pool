//! Simple example of using reqwest-proxy-pool.

use reqwest_middleware::ClientBuilder;
use reqwest_proxy_pool::{ProxyPoolMiddleware, ProxyPoolConfig, ProxySelectionStrategy};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Initializing proxy pool...");
    
    let config = ProxyPoolConfig::builder()
        // free socks5 proxy urls, format like `Free-Proxy`
        .sources(vec![
            "https://raw.githubusercontent.com/dpangestuw/Free-Proxy/main/socks5_proxies.txt",
        ])
        .health_check_timeout(Duration::from_secs(5))
        .health_check_url("https://www.example.com")
        .retry_count(2)
        .selection_strategy(ProxySelectionStrategy::FastestResponse)
        // rate limit for each proxy, lower performance but avoid banned
        .max_requests_per_second(3.0)
        .build();

    let proxy_pool = ProxyPoolMiddleware::new(config).await?;

    let client = ClientBuilder::new(reqwest::Client::new())
        .with(proxy_pool)
        .build();

    println!("Sending request...");
    let response = client.get("https://httpbin.org/ip").send().await?;
    
    println!("Status: {}", response.status());
    println!("Response: {}", response.text().await?);

    Ok(())
}
