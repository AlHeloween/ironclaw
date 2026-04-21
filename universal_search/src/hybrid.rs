//! Hybrid mode: combines Sourcegraph results with web search.

use crate::config::Config;
use crate::web::{execute_sourcegraph, execute_web_search, SourcegraphRequest, WebSearchRequest};
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
    pub sourcegraph_results: usize,
    pub web_results: usize,
    pub results: Vec<serde_json::Value>,
}

pub async fn execute_hybrid(
    http_client: &reqwest::Client,
    config: &Config,
    request: &HybridRequest,
) -> Result<HybridResponse, String> {
    let mut all_results: Vec<serde_json::Value> = Vec::new();
    let mut sg_count = 0usize;
    let mut web_count = 0usize;

    // 1. Sourcegraph code search
    let sg_request = SourcegraphRequest {
        query: request.query.clone(),
        url: request.url.clone(),
        count: request.count,
    };

    match execute_sourcegraph(http_client, &config.web_search.sourcegraph, &sg_request).await {
        Ok(response) => {
            sg_count = response.results.len();
            for result in response.results {
                if let Ok(mut value) = serde_json::to_value(&result) {
                    if let Some(o) = value.as_object_mut() {
                        o.insert("source".to_string(), serde_json::json!("sourcegraph"));
                    }
                    all_results.push(value);
                }
            }
        }
        Err(e) => {
            tracing::warn!("Sourcegraph search failed: {}", e);
        }
    }

    // 2. Web search via Firecrawl
    let web_request = WebSearchRequest {
        query: request.query.clone(),
        count: request.count,
        scrape_formats: None,
        only_main_content: None,
    };

    match execute_web_search(http_client, &config.web_search.firecrawl, &web_request).await {
        Ok(response) => {
            web_count = response.results.len();
            for result in response.results {
                if let Ok(mut value) = serde_json::to_value(&result) {
                    if let Some(o) = value.as_object_mut() {
                        o.insert("source".to_string(), serde_json::json!("web"));
                    }
                    all_results.push(value);
                }
            }
        }
        Err(e) => {
            tracing::warn!("Web search failed: {}", e);
        }
    }

    Ok(HybridResponse {
        query: request.query.clone(),
        mode: "hybrid".to_string(),
        result_count: all_results.len(),
        sourcegraph_results: sg_count,
        web_results: web_count,
        results: all_results,
    })
}
