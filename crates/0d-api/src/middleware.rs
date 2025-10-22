use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use governor::clock::QuantaInstant;
use governor::middleware::RateLimitingMiddleware;
use std::collections::HashSet;
use std::sync::Arc;
use tower_governor::governor::SharedRateLimiter;
use tower_governor::key_extractor::{KeyExtractor, SmartIpKeyExtractor};

pub struct RateLimitConfig<K, M>
where
    K: std::hash::Hash + Eq + Clone,
    M: RateLimitingMiddleware<QuantaInstant>,
{
    pub limiter: SharedRateLimiter<K, M>,
    pub whitelist_domains: Arc<HashSet<String>>,
}

// Manual Clone implementation since M might not implement Clone
impl<K, M> Clone for RateLimitConfig<K, M>
where
    K: std::hash::Hash + Eq + Clone,
    M: RateLimitingMiddleware<QuantaInstant>,
{
    fn clone(&self) -> Self {
        Self {
            limiter: self.limiter.clone(),
            whitelist_domains: self.whitelist_domains.clone(),
        }
    }
}

/// Extracts the domain from Origin or Referer header
fn extract_domain_from_headers(headers: &HeaderMap) -> Option<String> {
    // Try Origin header first
    if let Some(origin) = headers.get("origin") {
        if let Ok(origin_str) = origin.to_str() {
            return extract_domain(origin_str);
        }
    }

    // Fallback to Referer header
    if let Some(referer) = headers.get("referer") {
        if let Ok(referer_str) = referer.to_str() {
            return extract_domain(referer_str);
        }
    }

    None
}

/// Extracts domain from a URL string
/// Examples:
/// - "https://example.com/path" -> "example.com"
/// - "http://api.example.com:8080/path" -> "api.example.com"
fn extract_domain(url: &str) -> Option<String> {
    // Remove protocol if present
    let without_protocol = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Extract domain (before first '/' or ':' for port)
    let domain = without_protocol
        .split(&['/', ':', '?'][..])
        .next()?
        .to_lowercase();

    if domain.is_empty() {
        None
    } else {
        Some(domain)
    }
}

/// Middleware that checks if a request is from a whitelisted domain,
/// and if not, applies rate limiting.
///
/// This middleware is designed to work with IP-based rate limiting using SmartIpKeyExtractor.
pub async fn rate_limit_middleware<M>(
    config: RateLimitConfig<std::net::IpAddr, M>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>
where
    M: RateLimitingMiddleware<QuantaInstant>,
{
    // Check if request is from a whitelisted domain
    if let Some(domain) = extract_domain_from_headers(request.headers()) {
        if config.whitelist_domains.contains(&domain) {
            tracing::debug!(
                domain = %domain,
                "Request from whitelisted domain, bypassing rate limit"
            );
            return Ok(next.run(request).await);
        }
    }

    // Not whitelisted, apply rate limiting using the governor
    // Extract the key (IP address) from the request
    let key = match SmartIpKeyExtractor.extract(&request) {
        Ok(key) => key,
        Err(e) => {
            tracing::warn!(error = ?e, "Failed to extract IP for rate limiting");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match config.limiter.check_key(&key) {
        Ok(_) => {
            // Rate limit check passed
            Ok(next.run(request).await)
        }
        Err(_) => {
            // Rate limit exceeded
            tracing::warn!(
                key = ?key,
                "Rate limit exceeded"
            );

            // Return 429 Too Many Requests
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain("http://api.example.com:8080/path"),
            Some("api.example.com".to_string())
        );
        assert_eq!(
            extract_domain("example.com"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain("https://EXAMPLE.COM"),
            Some("example.com".to_string())
        );
        assert_eq!(extract_domain(""), None);
    }
}
