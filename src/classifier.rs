//! Body-aware classification for proxy health feedback.

/// Result of classifying a response body from a proxy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProxyBodyVerdict {
    /// Response is good. Proxy records a success.
    Success,
    /// Proxy is blocked (e.g. captcha, anti-bot). Records failure, retries with another proxy.
    ProxyBlocked,
    /// Server-side issue unrelated to proxy. Returns response as-is without affecting proxy stats.
    Passthrough,
}

/// Classify responses with full body to determine proxy health at business level.
///
/// # Example
/// ```rust,no_run
/// use reqwest_proxy_pool::{BodyClassifier, ProxyBodyVerdict};
///
/// struct CaptchaDetector;
///
/// impl BodyClassifier for CaptchaDetector {
///     fn classify(
///         &self,
///         status: reqwest::StatusCode,
///         _headers: &reqwest::header::HeaderMap,
///         body: &[u8],
///     ) -> ProxyBodyVerdict {
///         if status == reqwest::StatusCode::TOO_MANY_REQUESTS
///             || String::from_utf8_lossy(body).contains("captcha")
///         {
///             ProxyBodyVerdict::ProxyBlocked
///         } else if status.is_success() {
///             ProxyBodyVerdict::Success
///         } else {
///             ProxyBodyVerdict::Passthrough
///         }
///     }
/// }
/// ```
pub trait BodyClassifier: Send + Sync + 'static {
    fn classify(
        &self,
        status: reqwest::StatusCode,
        headers: &reqwest::header::HeaderMap,
        body: &[u8],
    ) -> ProxyBodyVerdict;
}

/// Default classifier: HTTP success = Success, otherwise Passthrough.
pub struct DefaultBodyClassifier;

impl BodyClassifier for DefaultBodyClassifier {
    fn classify(
        &self,
        status: reqwest::StatusCode,
        _headers: &reqwest::header::HeaderMap,
        _body: &[u8],
    ) -> ProxyBodyVerdict {
        if status.is_success() {
            ProxyBodyVerdict::Success
        } else {
            ProxyBodyVerdict::Passthrough
        }
    }
}
