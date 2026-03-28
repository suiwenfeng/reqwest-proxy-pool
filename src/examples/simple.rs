//! Simple example of using reqwest-proxy-pool.

use reqwest_middleware::ClientBuilder;
use reqwest_proxy_pool::{
    HostConfig, ProxyPoolConfig, ProxyPoolMiddleware, ProxyResponseVerdict, ProxySelectionStrategy,
    ResponseClassifier, RetryStrategy,
};
use std::time::Duration;

/// Custom classifier that detects captcha/anti-bot responses.
/// The middleware will automatically mark the proxy as failed and retry
/// with a different proxy when `ProxyBlocked` is returned.
struct CaptchaDetector;

impl ResponseClassifier for CaptchaDetector {
    fn classify(&self, response: &reqwest::Response) -> ProxyResponseVerdict {
        match response.status().as_u16() {
            403 | 429 => ProxyResponseVerdict::ProxyBlocked,
            500..=599 => ProxyResponseVerdict::Passthrough,
            _ => ProxyResponseVerdict::Success,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing proxy pool...");

    let api_host = HostConfig::builder("httpbin.org")
        .primary(true)
        .health_check_timeout(Duration::from_secs(5))
        .health_check_url("https://httpbin.org/ip")
        .retry_count(2)
        .retry_strategy(RetryStrategy::NewProxyOnRetry)
        .selection_strategy(ProxySelectionStrategy::FastestResponse)
        // minimum interval for each proxy instance to avoid bans
        .min_request_interval_ms(500)
        // custom response classifier to detect anti-bot/captcha blocking
        .response_classifier(CaptchaDetector)
        // accept invalid certs (needed for most free SOCKS5 proxies)
        .danger_accept_invalid_certs(true)
        .build();

    let static_host = HostConfig::builder("example.com")
        .health_check_url("https://example.com")
        .retry_count(1)
        .selection_strategy(ProxySelectionStrategy::Random)
        .min_request_interval_ms(800)
        .build();

    let config = ProxyPoolConfig::builder()
        // shared socks5 proxy sources for all host pools
        .sources(vec![
            "https://cdn.jsdelivr.net/gh/dpangestuw/Free-Proxy@main/socks5_proxies.txt",
            "https://cdn.jsdelivr.net/gh/proxifly/free-proxy-list@main/proxies/protocols/socks5/data.txt",
        ])
        .hosts(vec![api_host, static_host])
        .build();

    let proxy_pool = ProxyPoolMiddleware::new(config).await?;

    let client = ClientBuilder::new(reqwest::Client::new())
        .with(proxy_pool)
        .build();

    println!("Sending request to httpbin.org host pool...");
    let response = client.get("https://httpbin.org/ip").send().await?;

    println!("Status: {}", response.status());
    println!("Response: {}", response.text().await?);

    Ok(())
}
