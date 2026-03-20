//! Response classification for proxy health feedback.

/// Result of classifying a response from a proxy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProxyResponseVerdict {
    /// Response is good. Proxy records a success.
    Success,
    /// Proxy is blocked (e.g. captcha, anti-bot). Records failure, retries with another proxy.
    ProxyBlocked,
    /// Server-side issue unrelated to proxy. Returns response as-is without affecting proxy stats.
    Passthrough,
}

/// Classify responses to determine proxy health at the business level.
///
/// Implement this trait to detect anti-bot responses (captchas, blocks, etc.)
/// that pass HTTP-level health checks but indicate the proxy is unusable
/// for your target site.
///
/// # Example
/// ```rust,no_run
/// use reqwest_proxy_pool::{ResponseClassifier, ProxyResponseVerdict};
///
/// struct CaptchaDetector;
///
/// impl ResponseClassifier for CaptchaDetector {
///     fn classify(&self, response: &reqwest::Response) -> ProxyResponseVerdict {
///         // Check status or headers for signs of blocking
///         if response.status() == 403 {
///             ProxyResponseVerdict::ProxyBlocked
///         } else {
///             ProxyResponseVerdict::Success
///         }
///     }
/// }
/// ```
pub trait ResponseClassifier: Send + Sync + 'static {
    fn classify(&self, response: &reqwest::Response) -> ProxyResponseVerdict;
}

/// Default classifier: HTTP success = Success, otherwise Passthrough.
pub struct DefaultResponseClassifier;

impl ResponseClassifier for DefaultResponseClassifier {
    fn classify(&self, response: &reqwest::Response) -> ProxyResponseVerdict {
        if response.status().is_success() {
            ProxyResponseVerdict::Success
        } else {
            ProxyResponseVerdict::Passthrough
        }
    }
}
