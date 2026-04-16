//! Hybrid mode: combines local code search with Sourcegraph results.

use crate::config::FirecrawlSearchConfig;
use crate::sourcegraph::{execute_sourcegraph, SourcegraphRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HybridRequest {
    pub query: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub count: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct HybridResponse {
    pub query: String,
    pub mode: String,
    pub result_count: usize,
    pub local_results: usize,
    pub sourcegraph_results: usize,
    pub results: Vec<serde_json::Value>,
}

pub async fn execute_hybrid(
    config: &FirecrawlSearchConfig,
    request: &HybridRequest,
) -> Result<HybridResponse, String> {
    let mut all_results: Vec<serde_json::Value> = Vec::new();

    // 1. Try local code search first (if enabled)
    if config.local_search.enabled {
        match execute_local_search(&config.local_search.url, request).await {
            Ok(results) => all_results.extend(results),
            Err(e) => {
                tracing::warn!("Local code search failed: {}", e);
                // Continue with Sourcegraph even if local fails
            }
        }
    }

    // 2. Then Sourcegraph
    let sg_request = SourcegraphRequest {
        query: request.query.clone(),
        url: request.url.clone(),
        count: request.count,
    };

    match execute_sourcegraph(&config.sourcegraph, &sg_request).await {
        Ok(response) => {
            for result in response.results {
                if let Ok(value) = serde_json::to_value(result) {
                    all_results.push(value);
                }
            }
        }
        Err(e) => {
            tracing::warn!("Sourcegraph search failed: {}", e);
            // Continue with whatever local results we have
        }
    }

    let local_count = all_results
        .iter()
        .filter(|r| r.get("source").and_then(|s| s.as_str()) == Some("local"))
        .count();
    let sg_count = all_results
        .iter()
        .filter(|r| r.get("source").and_then(|s| s.as_str()) == Some("sourcegraph"))
        .count();

    Ok(HybridResponse {
        query: request.query.clone(),
        mode: "hybrid".to_string(),
        result_count: all_results.len(),
        local_results: local_count,
        sourcegraph_results: sg_count,
        results: all_results,
    })
}

async fn execute_local_search(
    local_url: &str,
    request: &HybridRequest,
) -> Result<Vec<serde_json::Value>, String> {
    let search_endpoint = format!("{}/search", local_url.trim_end_matches('/'));

    let request_body = serde_json::json!({
        "query": request.query,
        "limit": request.count.unwrap_or(5).clamp(1, 20),
    });

    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", reqwest::header::HeaderValue::from_static("application/json"));
    headers.insert("Content-Type", reqwest::header::HeaderValue::from_static("application/json"));
    headers.insert("User-Agent", reqwest::header::HeaderValue::from_static("IronClaw-Firecrawl-Service/1.0"));

    let body_str = request_body.to_string();
    let resp = client
        .post(&search_endpoint)
        .headers(headers.clone())
        .body(body_str)
        .send()
        .await
        .map_err(|e| format!("Local search request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Local search error (HTTP {})", resp.status()));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read local search response: {}", e))?;

    let local_response: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse local search response: {}", e))?;

    let results = local_response
        .get("results")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    // Add "source": "local" to each result
    let formatted: Vec<serde_json::Value> = results
        .into_iter()
        .map(|mut r| {
            if let Some(obj) = r.as_object_mut() {
                obj.insert("source".to_string(), serde_json::json!("local"));
            }
            r
        })
        .collect();

    Ok(formatted)
}
