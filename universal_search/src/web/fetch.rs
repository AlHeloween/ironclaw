//! Direct HTTPS content fetch — bypasses Firecrawl for simple URL retrieval.

use serde::{Deserialize, Serialize};

/// Max content size in bytes (10 MB).
const MAX_CONTENT_SIZE: u64 = 10 * 1024 * 1024;

/// Default timeout in seconds.
const TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct FetchResponse {
    pub url: String,
    pub status_code: u16,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub content_length: usize,
}

pub async fn execute_fetch(
    client: &reqwest::Client,
    request: &FetchRequest,
) -> Result<FetchResponse, String> {
    let url = &request.url;

    validate_url(url)?;

    let resp = client
        .get(url)
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| format!("Fetch failed: {}", e))?;

    let status_code = resp.status().as_u16();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Pre-check content length
    if let Some(len) = resp.content_length() {
        if len > MAX_CONTENT_SIZE {
            return Err(format!(
                "Content length {} exceeds maximum {} bytes ({} MB)",
                len,
                MAX_CONTENT_SIZE,
                MAX_CONTENT_SIZE / (1024 * 1024)
            ));
        }
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    if bytes.len() as u64 > MAX_CONTENT_SIZE {
        return Err(format!(
            "Content size {} exceeds maximum {} bytes",
            bytes.len(),
            MAX_CONTENT_SIZE
        ));
    }

    let content_length = bytes.len();
    let content = String::from_utf8(bytes.to_vec())
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).to_string());

    Ok(FetchResponse {
        url: url.clone(),
        status_code,
        content,
        content_type,
        content_length,
    })
}

/// Validate that the URL is HTTP/HTTPS and not pointing to localhost.
fn validate_url(url: &str) -> Result<(), String> {
    if url.len() > 2048 {
        return Err("URL exceeds maximum length of 2048 characters".to_string());
    }

    let (scheme, host) = parse_scheme_host(url)?;

    if scheme != "http" && scheme != "https" {
        return Err(format!(
            "Unsupported URL scheme: {}. Only http and https are allowed.",
            scheme
        ));
    }

    if is_localhost(host) {
        return Err("Requests to localhost/loopback addresses are not allowed".to_string());
    }

    Ok(())
}

fn parse_scheme_host(url: &str) -> Result<(&str, &str), String> {
    let rest = url
        .strip_prefix("https://")
        .map(|r| ("https", r))
        .or_else(|| url.strip_prefix("http://").map(|r| ("http", r)))
        .ok_or_else(|| "URL must start with http:// or https://".to_string())?;

    // rest is (scheme, remainder_of_url)
    let scheme = rest.0;
    let remainder = rest.1;

    let host = remainder
        .split('/')
        .next()
        .and_then(|h| h.split(':').next())
        .unwrap_or(remainder);

    if host.is_empty() {
        return Err("URL has no hostname".to_string());
    }

    Ok((scheme, host))
}

fn is_localhost(host: &str) -> bool {
    let host_lower = host.to_lowercase();
    host_lower == "localhost"
        || host_lower.starts_with("127.")
        || host_lower == "::1"
        || host_lower == "0.0.0.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_https_ok() {
        assert!(validate_url("https://example.com/path?q=1").is_ok());
    }

    #[test]
    fn test_validate_http_ok() {
        assert!(validate_url("http://example.com").is_ok());
    }

    #[test]
    fn test_validate_rejects_localhost() {
        assert!(validate_url("http://localhost:3000").is_err());
        assert!(validate_url("http://127.0.0.1:8080").is_err());
        assert!(validate_url("http://0.0.0.0").is_err());
        assert!(validate_url("http://::1").is_err());
    }

    #[test]
    fn test_validate_rejects_ftp() {
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_validate_rejects_no_scheme() {
        assert!(validate_url("example.com/page").is_err());
    }

    #[test]
    fn test_validate_too_long() {
        let long = format!("http://example.com/{}", "a".repeat(2050));
        assert!(validate_url(&long).is_err());
    }
}
