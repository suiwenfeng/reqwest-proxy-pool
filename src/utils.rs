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
            if line.starts_with("socks5://") || line.starts_with("socks5h://") {
                Some(line.to_string())
            } else if line.contains(':')
                && !line.contains("://")
                && !line.starts_with('#')
                && !line.is_empty()
            {
                // Try to parse IP:PORT format
                Some(format!("socks5://{}", line))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_proxy_list;

    #[test]
    fn parse_supports_socks5_and_socks5h() {
        let content = "socks5://127.0.0.1:1080\nsocks5h://127.0.0.2:1080\n";
        let parsed = parse_proxy_list(content);
        assert_eq!(
            parsed,
            vec![
                "socks5://127.0.0.1:1080".to_string(),
                "socks5h://127.0.0.2:1080".to_string()
            ]
        );
    }

    #[test]
    fn parse_rejects_invalid_socks5_prefix() {
        let content = "socks5x://127.0.0.1:1080\nnot-a-proxy\n";
        let parsed = parse_proxy_list(content);
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_plain_host_port_to_socks5() {
        let content = "1.2.3.4:1080\n# comment\n";
        let parsed = parse_proxy_list(content);
        assert_eq!(parsed, vec!["socks5://1.2.3.4:1080".to_string()]);
    }
}
