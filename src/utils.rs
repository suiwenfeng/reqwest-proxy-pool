//! Utility functions for the proxy pool.

use reqwest::Client;

/// Fetch and parse a list of proxies from a URL or file path.
pub(crate) async fn fetch_proxies_from_source(source: &str) -> Result<Vec<String>, reqwest::Error> {
    if source.starts_with("http") {
        // Fetch from URL
        let client = Client::new();
        let response = client.get(source).send().await?;
        let content = response.text().await?;
        Ok(parse_proxy_list(&content))
    } else {
        // Read from file
        match std::fs::read_to_string(source) {
            Ok(content) => Ok(parse_proxy_list(&content)),
            Err(_) => Ok(Vec::new()),
        }
    }
}

/// Parse the text content to extract SOCKS5 proxy URLs.
pub(crate) fn parse_proxy_list(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("socks5://") {
                Some(line.to_string())
            } else if line.contains(':') && !line.starts_with('#') && !line.is_empty() {
                // Try to parse IP:PORT format
                Some(format!("socks5://{}", line))
            } else {
                None
            }
        })
        .collect()
}
