//! Code search via Sourcegraph public API.

use crate::config::SourcegraphConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SourcegraphRequest {
    pub query: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SourcegraphResponse {
    data: Option<SourcegraphData>,
    errors: Option<Vec<SourcegraphError>>,
}

#[derive(Debug, Deserialize)]
struct SourcegraphData {
    search: Option<SourcegraphSearch>,
}

#[derive(Debug, Deserialize)]
struct SourcegraphSearch {
    results: Option<SourcegraphResults>,
}

#[derive(Debug, Deserialize)]
struct SourcegraphResults {
    results: Option<Vec<SourcegraphMatch>>,
    match_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SourcegraphMatch {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    match_type: Option<String>,
    file: Option<SourcegraphFile>,
    repository: Option<SourcegraphRepo>,
    line: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SourcegraphFile {
    path: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SourcegraphRepo {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SourcegraphError {
    message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SourcegraphResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct SourcegraphResponseOutput {
    pub query: String,
    pub mode: String,
    pub match_count: u32,
    pub results: Vec<SourcegraphResult>,
}

pub async fn execute_sourcegraph(
    config: &SourcegraphConfig,
    request: &SourcegraphRequest,
) -> Result<SourcegraphResponseOutput, String> {
    let search_query = if let Some(ref repo) = request.url {
        format!("repo:{} {}", repo, request.query)
    } else {
        request.query.clone()
    };

    let graphql_query = format!(
        r#"query {{ search(query: "{}") {{ results {{ results {{ ... on FileMatch {{ file {{ path name }} repository {{ name }} }} }} matchCount }} }} }}"#,
        search_query.replace('"', "\\\"")
    );

    let request_body = serde_json::json!({
        "query": graphql_query
    });

    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", reqwest::header::HeaderValue::from_static("application/json"));
    headers.insert("Content-Type", reqwest::header::HeaderValue::from_static("application/json"));
    headers.insert("User-Agent", reqwest::header::HeaderValue::from_static("IronClaw-Firecrawl-Service/1.0"));

    if let Some(ref token) = config.access_token {
        if !token.is_empty() {
            let auth_value = reqwest::header::HeaderValue::from_str(&format!("token {}", token))
                .map_err(|e| format!("Invalid auth header: {}", e))?;
            headers.insert("Authorization", auth_value);
        }
    }

    let body_str = request_body.to_string();
    let api_url = config.api_url.trim_end_matches('/');
    let resp = client
        .post(api_url)
        .headers(headers.clone())
        .body(body_str)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let sg_response: SourcegraphResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Sourcegraph response: {}", e))?;

    if let Some(errors) = &sg_response.errors {
        if !errors.is_empty() {
            let msgs: Vec<_> = errors.iter().filter_map(|e| e.message.clone()).collect();
            return Err(format!("Sourcegraph API error: {}", msgs.join("; ")));
        }
    }

    let search_results = sg_response
        .data
        .and_then(|d| d.search)
        .and_then(|s| s.results);

    let match_count = search_results
        .as_ref()
        .and_then(|r| r.match_count)
        .unwrap_or(0);

    let results = search_results.and_then(|r| r.results).unwrap_or_default();

    let formatted: Vec<SourcegraphResult> = results
        .into_iter()
        .filter_map(|m| {
            let mut entry = SourcegraphResult {
                repository: None,
                path: None,
                file: None,
                line: m.line,
                source: "sourcegraph".to_string(),
            };

            if let Some(repo) = &m.repository {
                if let Some(name) = &repo.name {
                    entry.repository = Some(name.clone());
                }
            }

            if let Some(file) = &m.file {
                if let Some(path) = &file.path {
                    entry.path = Some(path.clone());
                }
                if let Some(name) = &file.name {
                    entry.file = Some(name.clone());
                }
            }

            if entry.repository.is_none() && entry.path.is_none() && entry.file.is_none() {
                return None;
            }

            Some(entry)
        })
        .collect();

    Ok(SourcegraphResponseOutput {
        query: request.query.clone(),
        mode: "sourcegraph".to_string(),
        match_count,
        results: formatted,
    })
}
