//! Web search via Firecrawl API.

use crate::config::FirecrawlConfig;
use serde::{Deserialize, Serialize};

const DEFAULT_COUNT: u32 = 5;
const MAX_COUNT: u32 = 20;
const MAX_RETRIES: u32 = 3;

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSearchRequest {
    pub query: String,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub scrape_formats: Option<String>,
    #[serde(default)]
    pub only_main_content: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FirecrawlSearchResponse {
    success: Option<bool>,
    data: Option<FirecrawlSearchData>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FirecrawlSearchData {
    results: Vec<FirecrawlSearchResult>,
}

#[derive(Debug, Deserialize)]
struct FirecrawlSearchResult {
    url: Option<String>,
    title: Option<String>,
    description: Option<String>,
    markdown: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WebSearchResponse {
    pub query: String,
    pub mode: String,
    pub result_count: usize,
    pub results: Vec<WebSearchResult>,
}

pub async fn execute_web_search(
    config: &FirecrawlConfig,
    request: &WebSearchRequest,
) -> Result<WebSearchResponse, String> {
    let count = request.count.unwrap_or(DEFAULT_COUNT).clamp(1, MAX_COUNT);
    let api_url = config.api_url.trim_end_matches('/');
    let search_endpoint = format!("{}/v1/search", api_url);

    let mut request_body = serde_json::json!({
        "query": request.query,
        "limit": count,
        "scrapeOptions": {}
    });

    if let Some(ref formats) = request.scrape_formats {
        let formats_vec: Vec<&str> = formats.split(',').collect();
        request_body["scrapeOptions"]["formats"] = serde_json::json!(formats_vec);
    } else {
        request_body["scrapeOptions"]["formats"] = serde_json::json!(["markdown"]);
    }

    if let Some(only_main) = request.only_main_content {
        request_body["scrapeOptions"]["onlyMainContent"] = serde_json::json!(only_main);
    }

    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", reqwest::header::HeaderValue::from_static("application/json"));
    headers.insert("Content-Type", reqwest::header::HeaderValue::from_static("application/json"));
    headers.insert("User-Agent", reqwest::header::HeaderValue::from_static("IronClaw-Firecrawl-Service/1.0"));

    if let Some(ref key) = config.api_key {
        if !key.is_empty() {
            let auth_value = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", key))
                .map_err(|e| format!("Invalid auth header: {}", e))?;
            headers.insert("Authorization", auth_value);
        }
    }

    let body_str = request_body.to_string();
    let mut attempt = 0;
    let mut response = None;

    while attempt < MAX_RETRIES {
        attempt += 1;

        let resp = client
            .post(&search_endpoint)
            .headers(headers.clone())
            .body(body_str.clone())
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if resp.status().is_success() {
            response = Some(resp);
            break;
        }

        if attempt < MAX_RETRIES && (resp.status() == 429 || resp.status().is_server_error()) {
            tracing::warn!(
                "Firecrawl error {} (attempt {}/{}). Retrying...",
                resp.status(),
                attempt,
                MAX_RETRIES
            );
            tokio::time::sleep(std::time::Duration::from_secs(2_u64.pow(attempt - 1))).await;
            continue;
        }

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;
        return Err(format!("Firecrawl error (HTTP {}): {}", status, body));
    }

    let resp = response.ok_or("No response received")?;
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let fc_response: FirecrawlSearchResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Firecrawl response: {}", e))?;

    if let Some(err) = fc_response.error {
        return Err(format!("Firecrawl API error: {}", err));
    }

    let results = fc_response.data.map(|d| d.results).unwrap_or_default();

    let formatted: Vec<WebSearchResult> = results
        .into_iter()
        .filter_map(|r| {
            let url = r.url.clone()?;
            let title = r.title.unwrap_or_default();

            let entry = WebSearchResult {
                title,
                url,
                description: r.description.clone(),
                markdown: r.markdown.clone(),
                site_name: extract_hostname(&r.url.clone().unwrap_or_default()),
            };

            Some(entry)
        })
        .collect();

    Ok(WebSearchResponse {
        query: request.query.clone(),
        mode: "search".to_string(),
        result_count: formatted.len(),
        results: formatted,
    })
}

fn extract_hostname(url: &str) -> Option<String> {
    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = after_scheme.split('/').next()?;
    let host = host.split(':').next()?;
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}
