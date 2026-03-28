# reqwest-proxy-pool

Proxy pool middleware implementation for [`reqwest-middleware`](https://crates.io/crates/reqwest-middleware).

[![Crates.io](https://img.shields.io/crates/v/reqwest-proxy-pool.svg)](https://crates.io/crates/reqwest-proxy-pool)
[![Docs.rs](https://docs.rs/reqwest-proxy-pool/badge.svg)](https://docs.rs/reqwest-proxy-pool)
[![CI](https://github.com/suiwenfeng/reqwest-proxy-pool/actions/workflows/ci.yml/badge.svg)](https://github.com/suiwenfeng/reqwest-proxy-pool/actions/workflows/ci.yml)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

## Features

### ✨ Comprehensive Proxy Support

- Automatic parsing of free SOCKS5/SOCKS5H proxies from multiple sources
- Per-host proxy pools with independent health-check policies

### ⚡ Intelligent Proxy Management

- Multiple proxy selection strategies (FastestResponse, MostReliable, RoundRobin, Random)
- Per-proxy minimum request interval to avoid bans
- Automatic retry mechanism for failed requests
- Retry strategy control (`DefaultSelection` / `NewProxyOnRetry`)
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
reqwest-proxy-pool = "0.3"
reqwest-middleware = "0.5"
tokio = { version = "1", features = ["full"] }
```

### Usage

```rust
use reqwest_middleware::ClientBuilder;
use reqwest_proxy_pool::{
    HostConfig, ProxyPoolConfig, ProxyPoolMiddleware, ProxyResponseVerdict,
    ProxySelectionStrategy, ResponseClassifier, RetryStrategy,
};
use std::time::Duration;

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
    let api_host = HostConfig::builder("httpbin.org")
        .primary(true)
        .health_check_timeout(Duration::from_secs(5))
        .health_check_url("https://httpbin.org/ip")
        .retry_count(2)
        .retry_strategy(RetryStrategy::NewProxyOnRetry)
        .selection_strategy(ProxySelectionStrategy::FastestResponse)
        .min_request_interval_ms(500)
        .response_classifier(CaptchaDetector)
        .danger_accept_invalid_certs(true)
        .build();

    let static_host = HostConfig::builder("example.com")
        .health_check_url("https://example.com")
        .retry_count(1)
        .selection_strategy(ProxySelectionStrategy::Random)
        .min_request_interval_ms(800)
        .build();

    let config = ProxyPoolConfig::builder()
        // Shared proxy source list for all host pools.
        .sources(vec![
            "https://cdn.jsdelivr.net/gh/dpangestuw/Free-Proxy@main/socks5_proxies.txt",
            "https://cdn.jsdelivr.net/gh/proxifly/free-proxy-list@main/proxies/protocols/socks5/data.txt",
        ])
        // One host config = one dedicated pool.
        .hosts(vec![api_host, static_host])
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

### Configuration Options

`ProxyPoolConfig`:

| Option         | Description                                                    | Default   |
|----------------|----------------------------------------------------------------|-----------|
| `sources`      | List of URLs providing proxy lists (shared by all host pools) | Required  |
| `hosts`        | List of `HostConfig` (one host = one pool)                    | Required  |

`HostConfig`:

| Option                    | Description                                      | Default                    |
|---------------------------|--------------------------------------------------|----------------------------|
| `host`                    | Target host for this pool                        | Required                   |
| `primary`                 | Whether this host is fallback primary (exactly one must be `true`) | `false`                    |
| `health_check_interval`   | Interval for background health checks            | 300s                       |
| `health_check_timeout`    | Timeout for proxy health checks                  | 10s                        |
| `min_available_proxies`   | Min available proxies                            | 3                          |
| `health_check_url`        | URL to test proxy health                         | `"https://www.google.com"` |
| `retry_count`             | Number of retries for failed requests            | 3                          |
| `retry_strategy`          | Retry behavior                                   | `DefaultSelection`         |
| `selection_strategy`      | Proxy selection algorithm                        | `FastestResponse`          |
| `min_request_interval_ms` | Min interval per proxy request                   | 500                        |
| `response_classifier`     | Custom response classifier for proxy health      | `DefaultResponseClassifier` |
| `danger_accept_invalid_certs` | Accept invalid TLS certs (needed for most free proxies) | `false` |

### Host-Based Routing (Multiple Pools)

```rust
use reqwest_proxy_pool::{HostConfig, ProxyPoolConfig, ProxyPoolMiddleware};

let api_host = HostConfig::builder("api.example.com").build();
let web_host = HostConfig::builder("www.example.com").primary(true).build();

let config = ProxyPoolConfig::builder()
    .sources(vec!["https://example.com/shared-proxies.txt"])
    .hosts(vec![api_host, web_host])
    .build();

let middleware = ProxyPoolMiddleware::new(config).await?;
```

### Routing Rules

1. Request host matches a configured `HostConfig.host` -> use that host pool.
2. Request host does not match -> use the unique `HostConfig` with `primary(true)`.

`primary=true` is required for exactly one host.

### Migration (0.2 -> 0.3)

- `ProxyPoolConfig` (single pool config) -> split into:
  - top-level `ProxyPoolConfig { sources, hosts }`
  - per-host `HostConfig`
- `max_requests_per_second` -> `min_request_interval_ms`
- `retry_strategy` added:
  - `DefaultSelection`: previous behavior
  - `NewProxyOnRetry`: force different proxy on retries

Example migration:

```rust
// v0.2
// ProxyPoolConfig::builder().health_check_url(...).max_requests_per_second(3.0)

// v0.3
let host = HostConfig::builder("target.example.com")
    .primary(true)
    .health_check_url("https://target.example.com/health")
    .min_request_interval_ms(333)
    .build();
let config = ProxyPoolConfig::builder()
    .sources(vec!["https://.../socks5.txt"])
    .add_host(host)
    .build();
```

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
