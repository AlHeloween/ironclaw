//! URL scraping via Firecrawl API.

use crate::config::FirecrawlConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ContextRequest {
    pub query: String,
    pub url: String,
    #[serde(default)]
    pub scrape_formats: Option<String>,
    #[serde(default)]
    pub only_main_content: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FirecrawlScrapeResponse {
    success: Option<bool>,
    data: Option<FirecrawlScrapeData>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FirecrawlScrapeData {
    markdown: Option<String>,
    html: Option<String>,
    title: Option<String>,
    description: Option<String>,
    links: Option<Vec<String>>,
    screenshot: Option<String>,
    #[serde(rename = "sourceURL")]
    source_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ContextResponse {
    pub query: String,
    pub mode: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<String>>,
}

pub async fn execute_context(
    client: &reqwest::Client,
    config: &FirecrawlConfig,
    request: &ContextRequest,
) -> Result<ContextResponse, String> {
    if request.url.len() > 2048 {
        return Err("'url' exceeds maximum length of 2048 characters".into());
    }

    let api_url = config.api_url.trim_end_matches('/');
    let scrape_endpoint = format!("{}/v1/scrape", api_url);

    let mut request_body = serde_json::json!({
        "url": &request.url,
        "formats": ["markdown"]
    });

    if let Some(ref formats) = request.scrape_formats {
        let formats_vec: Vec<&str> = formats.split(',').collect();
        request_body["formats"] = serde_json::json!(formats_vec);
    }

    if let Some(only_main) = request.only_main_content {
        request_body["onlyMainContent"] = serde_json::json!(only_main);
    } else {
        request_body["onlyMainContent"] = serde_json::json!(true);
    }

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", reqwest::header::HeaderValue::from_static("application/json"));
    headers.insert("Content-Type", reqwest::header::HeaderValue::from_static("application/json"));
    headers.insert("User-Agent", reqwest::header::HeaderValue::from_static("Universal-Search-Service/1.0"));

    if let Some(ref key) = config.api_key {
        if !key.is_empty() {
            let auth_value = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", key))
                .map_err(|e| format!("Invalid auth header: {}", e))?;
            headers.insert("Authorization", auth_value);
        }
    }

    let body_str = request_body.to_string();
    let resp = client
        .post(&scrape_endpoint)
        .headers(headers.clone())
        .body(body_str)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
        return Err(format!("Firecrawl error (HTTP {}): {}", status, body));
    }

    let body = resp.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let fc_response: FirecrawlScrapeResponse = serde_json::from_str(&body).map_err(|e| format!("Failed to parse Firecrawl scrape response: {}", e))?;

    if let Some(err) = fc_response.error {
        return Err(format!("Firecrawl API error: {}", err));
    }

    let data = fc_response.data.ok_or_else(|| "Firecrawl returned no data for the requested URL".to_string())?;

    Ok(ContextResponse {
        query: request.query.clone(),
        mode: "context".to_string(),
        url: request.url.clone(),
        markdown: data.markdown,
        title: data.title,
        description: data.description,
        source_url: data.source_url,
        links: data.links,
    })
}
