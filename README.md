# reqwest-proxy-pool

Proxy pool middleware implementation for [`reqwest-middleware`](https://crates.io/crates/reqwest-middleware).

[![Crates.io](https://img.shields.io/crates/v/reqwest-proxy-pool.svg)](https://crates.io/crates/reqwest-proxy-pool)
[![Docs.rs](https://docs.rs/reqwest-proxy-pool/badge.svg)](https://docs.rs/reqwest-proxy-pool)
[![CI](https://github.com/suiwenfeng/reqwest-proxy-pool/actions/workflows/ci.yml/badge.svg)](https://github.com/suiwenfeng/reqwest-proxy-pool/actions/workflows/ci.yml)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

## Features

### ✨ Comprehensive Proxy Support

- Automatic parsing of free SOCKS5/SOCKS5H proxies from multiple sources
- Built-in health checking with customizable timeout and test URL

### ⚡ Intelligent Proxy Management

- Multiple proxy selection strategies (FastestResponse, RoundRobin, Random)
- Per-proxy rate limiting to avoid bans
- Automatic retry mechanism for failed requests
- Custom response classifier for business-level proxy health (anti-bot/captcha detection)

### 🔧 Easy Configuration

- Simple builder pattern for configuration
- Seamless integration with reqwest middleware stack

## Quickstart

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
reqwest = "0.13"
reqwest-proxy-pool = "0.1"
reqwest-middleware = "0.5"
tokio = { version = "1", features = ["full"] }
```

### Usage

```rust
use reqwest_middleware::ClientBuilder;
use reqwest_proxy_pool::{ProxyPoolConfig, ProxyPoolMiddleware, ProxySelectionStrategy};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ProxyPoolConfig::builder()
        .sources(vec![
            "https://raw.githubusercontent.com/dpangestuw/Free-Proxy/main/socks5_proxies.txt",
        ])
        .health_check_timeout(Duration::from_secs(5))
        .health_check_url("https://www.example.com")
        .retry_count(2)
        .selection_strategy(ProxySelectionStrategy::FastestResponse)
        .max_requests_per_second(3.0)
        .build();

    let proxy_pool = ProxyPoolMiddleware::new(config).await?;

    let client = ClientBuilder::new(reqwest::Client::new())
        .with(proxy_pool)
        .build();

    let response = client.get("https://httpbin.org/ip").send().await?;
    println!("Status: {}", response.status());
    println!("Response: {}", response.text().await?);

    Ok(())
}
```

### Custom Response Classifier

Detect anti-bot/captcha responses and automatically retry with a different proxy:

```rust
use reqwest_proxy_pool::{ResponseClassifier, ProxyResponseVerdict};

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

let config = ProxyPoolConfig::builder()
    .sources(vec!["..."])
    .response_classifier(CaptchaDetector)
    .build();
```

| Verdict | Effect |
|---------|--------|
| `Success` | Proxy records a success |
| `ProxyBlocked` | Proxy records a failure, retries with another proxy |
| `Passthrough` | Returns response as-is, proxy stats unaffected |

### Configuration Options

| Option                    | Description                           | Default                    |
|---------------------------|---------------------------------------|----------------------------|
| `sources`                 | List of URLs providing proxy lists    | Required                   |
| `health_check_interval`   | Interval for background health checks | 300s                       |
| `health_check_timeout`    | Timeout for proxy health checks       | 10s                        |
| `min_available_proxies`   | Min available proxies                 | 3                          |
| `health_check_url`        | URL to test proxy health              | `"https://www.google.com"` |
| `retry_count`             | Number of retries for failed requests | 3                          |
| `selection_strategy`      | Proxy selection algorithm             | `FastestResponse`          |
| `max_requests_per_second` | Rate limit per proxy                  | 5.0                        |
| `response_classifier`     | Custom response classifier for proxy health | `DefaultResponseClassifier` |
| `danger_accept_invalid_certs` | Accept invalid TLS certs (needed for most free proxies) | `false` |

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version 2.0</a>
or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
</sub>
