//! Simple example of using reqwest-proxy-pool.

use reqwest_middleware::ClientBuilder;
use reqwest_proxy_pool::{
    BodyClassifier, HostConfig, ProxyBodyVerdict, ProxyPoolConfig, ProxyPoolMiddleware,
    ProxySelectionStrategy, RetryStrategy,
};
use std::time::Duration;

/// Body classifier that detects captcha/anti-bot responses.
struct CaptchaDetector;

impl BodyClassifier for CaptchaDetector {
    fn classify(
        &self,
        status: reqwest::StatusCode,
        _headers: &reqwest::header::HeaderMap,
        body: &[u8],
    ) -> ProxyBodyVerdict {
        let body_text = String::from_utf8_lossy(body);
        if matches!(status.as_u16(), 403 | 429) || body_text.contains("captcha") {
            ProxyBodyVerdict::ProxyBlocked
        } else if (500..=599).contains(&status.as_u16()) {
            ProxyBodyVerdict::Passthrough
        } else {
            ProxyBodyVerdict::Success
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
        .selection_strategy(ProxySelectionStrategy::TopKReliableRandom)
        .reliable_top_k(8)
        // minimum interval for each proxy instance to avoid bans
        .min_request_interval_ms(500)
        // custom body classifier to detect anti-bot/captcha blocking
        .body_classifier(CaptchaDetector)
        .proxy_cooldown(Duration::from_secs(30))
        .build();

    let static_host = HostConfig::builder("example.com")
        .health_check_url("https://example.com")
        .retry_count(1)
        .selection_strategy(ProxySelectionStrategy::Random)
        .min_request_interval_ms(800)
        .build();

    let config = ProxyPoolConfig::builder()
        .client_builder_factory(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(12))
                .pool_idle_timeout(Duration::from_secs(30))
        })
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
