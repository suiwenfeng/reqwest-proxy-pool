# reqwest-proxy-pool

proxy pool middleware implementation for
[`reqwest-middleware`](https://crates.io/crates/reqwest-middleware).

[![Crates.io](https://img.shields.io/crates/v/reqwest-proxy-pool.svg)](https://crates.io/crates/reqwest-proxy-pool)
[![Docs.rs](https://docs.rs/reqwest-proxy-pool/badge.svg)](https://docs.rs/reqwest-proxy-pool)
<!-- [![Coverage Status](https://coveralls.io/repos/github/suiwenfeng/reqwest-proxy-pool/badge.svg?branch=main&t=UWgSpm)](https://coveralls.io/github/suiwenfeng/reqwest-proxy-pool?branch=main) -->

## Features

### ✨ Comprehensive Proxy Support

- Automatic parsing of free SOCKS5 proxies from multiple sources

- Built-in health checking with customizable timeout and test URL

### ⚡ Intelligent Proxy Management

- Multiple proxy selection strategies (FastestResponse, RoundRobin, Random)

- Per-proxy rate limiting to avoid bans

- Automatic retry mechanism for failed requests

### 🔧 Easy Configuration

- Simple builder pattern for configuration

- Seamless integration with reqwest middleware stack

## Quickstart

### Installation

- Add to your Cargo.toml:

```
[dependencies]
reqwest-proxy-pool = "0.1.2"
reqwest-middleware = "0.4.2"
tokio = { version = "1", features = ["full"] }
```

### Usage

``` Rust
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
```

### Configuration Options

| Option                   | Description                          | Default                     |
|--------------------------|--------------------------------------|-----------------------------|
| `sources`                | List of URLs providing proxy lists   | Required                    |
| `health_check_interval`  | Interval for background health checks| 300s                        |
| `health_check_timeout`   | Timeout for proxy health checks      | 10s                         |
| `min_available_proxies`  | Min available proxies                | 3                           |
| `health_check_url`       | URL to test proxy health             | `"https://www.google.com"`  |
| `retry_count`            | Number of retries for failed requests| 3                           |
| `selection_strategy`     | Proxy selection algorithm            | `FastestResponse`           |
| `max_requests_per_second`| Rate limit per proxy                 | 5 requests per second                       |

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
</sub>
