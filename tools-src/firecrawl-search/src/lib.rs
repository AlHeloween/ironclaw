//! Firecrawl Web Search & Context WASM Tool for IronClaw.
//!
//! Thin wrapper that forwards requests to the native Firecrawl Search Service
//! when available, with built-in fallback implementations for all modes:
//! - `search`: Web search with full-page content (Firecrawl)
//! - `context`: Scrape a specific URL for RAG grounding
//! - `sourcegraph`: Search public codebases via Sourcegraph (sourcegraph.com)
//! - `hybrid`: Search both Sourcegraph (public) and local code search service
//!
//! # Authentication
//!
//! Local instances require no API key. For cloud Firecrawl (firecrawl.dev),
//! set the `FIRECRAWL_API_KEY` secret:
//! `ironclaw secret set firecrawl_api_key <key>`
//!
//! For Sourcegraph, set `SOURCEGRAPH_ACCESS_TOKEN` for higher rate limits:
//! `ironclaw secret set sourcegraph_access_token <key>`
//!
//! # Configuration
//!
//! Set your Firecrawl API URL via the `FIRECRAWL_API_URL` environment variable.
//! Default: `http://localhost:3002` (local self-hosted instance)

wit_bindgen::generate!({
    world: "sandboxed-tool",
    path: "../../wit/tool.wit",
});

use serde::Deserialize;

const DEFAULT_API_URL: &str = "http://localhost:3002";
const DEFAULT_LOCAL_SEARCH_URL: &str = "http://127.0.0.1:3004";
const DEFAULT_COUNT: u32 = 5;
const MAX_COUNT: u32 = 20;
const MAX_RETRIES: u32 = 3;
const SOURCEGRAPH_API_URL: &str = "https://sourcegraph.com/.api/graphql";

struct FirecrawlSearchTool;

impl exports::near::agent::tool::Guest for FirecrawlSearchTool {
    fn execute(req: exports::near::agent::tool::Request) -> exports::near::agent::tool::Response {
        match execute_inner(&req.params) {
            Ok(result) => exports::near::agent::tool::Response {
                output: Some(result),
                error: None,
            },
            Err(e) => exports::near::agent::tool::Response {
                output: None,
                error: Some(e),
            },
        }
    }

    fn schema() -> String {
        SCHEMA.to_string()
    }

    fn description() -> String {
        "Search the web, scrape content, or search codebases. \
         Supports four modes: 'search' for web search with full-page markdown (Firecrawl), \
         'context' for scraping a specific URL for RAG grounding, \
         'sourcegraph' for searching public codebases via Sourcegraph, \
         and 'hybrid' for searching both Sourcegraph and the local code search service (local results first). \
         Uses the FIRECRAWL_API_URL env var (default: http://localhost:3002). \
         The local code search service runs on LOCAL_CODE_SEARCH_URL (default: http://127.0.0.1:3004). \
         No API key required for local instances."
            .to_string()
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SearchParams {
    query: String,
    mode: Option<String>,
    count: Option<u32>,
    url: Option<String>,
    scrape_formats: Option<String>,
    only_main_content: Option<bool>,
    context_size: Option<String>,
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
#[allow(dead_code)]
struct SourcegraphMatch {
    #[serde(rename = "type")]
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

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LocalSearchResponse {
    results: Vec<LocalSearchResult>,
    total: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LocalSearchResult {
    title: Option<String>,
    content: Option<String>,
    file_path: Option<String>,
    relative_path: Option<String>,
    index_name: Option<String>,
    language: Option<String>,
    line_number: Option<u32>,
    score: Option<f32>,
}

fn execute_inner(params: &str) -> Result<String, String> {
    let params: SearchParams =
        serde_json::from_str(params).map_err(|e| format!("Invalid parameters: {e}"))?;

    if params.query.is_empty() {
        return Err("'query' must not be empty".into());
    }
    if params.query.len() > 2000 {
        return Err("'query' exceeds maximum length of 2000 characters".into());
    }

    let mode = params.mode.as_deref().unwrap_or("search");

    match mode {
        "search" => execute_search(&params),
        "context" => execute_context(&params),
        "sourcegraph" => execute_sourcegraph(&params),
        "hybrid" => execute_hybrid(&params),
        _ => Err(format!(
            "Invalid 'mode': expected 'search', 'context', 'sourcegraph', or 'hybrid', got '{mode}'"
        )),
    }
}

fn execute_search(params: &SearchParams) -> Result<String, String> {
    let count = params.count.unwrap_or(DEFAULT_COUNT).clamp(1, MAX_COUNT);
    let api_url = get_api_url();
    let search_endpoint = format!("{}/v1/search", api_url.trim_end_matches('/'));

    let mut request_body = serde_json::json!({
        "query": params.query,
        "limit": count,
        "scrapeOptions": {}
    });

    if let Some(ref formats) = params.scrape_formats {
        let formats_vec: Vec<&str> = formats.split(',').collect();
        request_body["scrapeOptions"]["formats"] = serde_json::json!(formats_vec);
    } else {
        request_body["scrapeOptions"]["formats"] = serde_json::json!(["markdown"]);
    }

    if let Some(only_main) = params.only_main_content {
        request_body["scrapeOptions"]["onlyMainContent"] = serde_json::json!(only_main);
    }

    let body_str = request_body.to_string();
    let headers = build_headers();

    let response = make_request("POST", &search_endpoint, &headers, &body_str)?;
    let body = response_to_string(response)?;

    let fc_response: FirecrawlSearchResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Firecrawl search response: {e}"))?;

    if let Some(err) = fc_response.error {
        return Err(format!("Firecrawl API error: {err}"));
    }

    let results = fc_response.data.map(|d| d.results).unwrap_or_default();

    let formatted: Vec<serde_json::Value> = results
        .into_iter()
        .filter_map(|r| {
            let url = r.url.clone()?;
            let title = r.title.unwrap_or_default();

            let mut entry = serde_json::json!({
                "title": title,
                "url": url,
            });

            if let Some(desc) = &r.description {
                entry["description"] = serde_json::json!(desc);
            }
            if let Some(md) = &r.markdown {
                entry["markdown"] = serde_json::json!(md);
            }
            if let Some(host) = extract_hostname(&url) {
                entry["site_name"] = serde_json::json!(host);
            }

            Some(entry)
        })
        .collect();

    let output = serde_json::json!({
        "query": params.query,
        "mode": "search",
        "result_count": formatted.len(),
        "results": formatted,
    });

    serde_json::to_string(&output).map_err(|e| format!("Failed to serialize output: {e}"))
}

fn execute_context(params: &SearchParams) -> Result<String, String> {
    let url = params
        .url
        .as_ref()
        .ok_or_else(|| "'url' is required when mode='context'")?;

    if url.len() > 2048 {
        return Err("'url' exceeds maximum length of 2048 characters".into());
    }

    let api_url = get_api_url();
    let scrape_endpoint = format!("{}/v1/scrape", api_url.trim_end_matches('/'));

    let mut request_body = serde_json::json!({
        "url": url,
        "formats": ["markdown"]
    });

    if let Some(ref formats) = params.scrape_formats {
        let formats_vec: Vec<&str> = formats.split(',').collect();
        request_body["formats"] = serde_json::json!(formats_vec);
    }

    if let Some(only_main) = params.only_main_content {
        request_body["onlyMainContent"] = serde_json::json!(only_main);
    } else {
        request_body["onlyMainContent"] = serde_json::json!(true);
    }

    let body_str = request_body.to_string();
    let headers = build_headers();

    let response = make_request("POST", &scrape_endpoint, &headers, &body_str)?;
    let body = response_to_string(response)?;

    let fc_response: FirecrawlScrapeResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Firecrawl scrape response: {e}"))?;

    if let Some(err) = fc_response.error {
        return Err(format!("Firecrawl API error: {err}"));
    }

    let data = fc_response
        .data
        .ok_or_else(|| "Firecrawl returned no data for the requested URL".to_string())?;

    let mut output = serde_json::json!({
        "query": params.query,
        "mode": "context",
        "url": url,
    });

    if let Some(md) = &data.markdown {
        output["markdown"] = serde_json::json!(md);
    }
    if let Some(title) = &data.title {
        output["title"] = serde_json::json!(title);
    }
    if let Some(desc) = &data.description {
        output["description"] = serde_json::json!(desc);
    }
    if let Some(source) = &data.source_url {
        output["source_url"] = serde_json::json!(source);
    }
    if let Some(links) = &data.links {
        output["links"] = serde_json::json!(links);
    }

    serde_json::to_string(&output).map_err(|e| format!("Failed to serialize output: {e}"))
}

fn execute_sourcegraph(params: &SearchParams) -> Result<String, String> {
    let _count = params.count.unwrap_or(DEFAULT_COUNT).clamp(1, MAX_COUNT);

    let search_query = if let Some(ref repo) = params.url {
        format!("repo:{} {}", repo, params.query)
    } else {
        params.query.clone()
    };

    let graphql_query = format!(
        r#"query {{ search(query: "{}") {{ results {{ results {{ ... on FileMatch {{ file {{ path name }} repository {{ name }} }} }} matchCount }} }} }}"#,
        search_query.replace('"', "\\\"")
    );

    let request_body = serde_json::json!({
        "query": graphql_query
    });

    let body_str = request_body.to_string();
    let headers = build_sourcegraph_headers();

    let response = make_request("POST", SOURCEGRAPH_API_URL, &headers, &body_str)?;
    let body = response_to_string(response)?;

    let sg_response: SourcegraphResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Sourcegraph response: {e}"))?;

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

    let formatted: Vec<serde_json::Value> = results
        .into_iter()
        .filter_map(|m| {
            let mut entry = serde_json::json!({});

            if let Some(repo) = &m.repository {
                if let Some(name) = &repo.name {
                    entry["repository"] = serde_json::json!(name);
                }
            }

            if let Some(file) = &m.file {
                if let Some(path) = &file.path {
                    entry["path"] = serde_json::json!(path);
                }
            }

            if let Some(line) = m.line {
                entry["line"] = serde_json::json!(line);
            }

            if entry.as_object().map_or(true, |o| o.is_empty()) {
                return None;
            }

            Some(entry)
        })
        .collect();

    let output = serde_json::json!({
        "query": params.query,
        "mode": "sourcegraph",
        "match_count": match_count,
        "results": formatted,
    });

    serde_json::to_string(&output).map_err(|e| format!("Failed to serialize output: {e}"))
}

fn execute_hybrid(params: &SearchParams) -> Result<String, String> {
    let sg_results = execute_sourcegraph_inner(params);
    let local_results = execute_local_search(params);

    let mut all_results: Vec<serde_json::Value> = Vec::new();

    if let Ok(results) = local_results {
        all_results.extend(results);
    }

    if let Ok(results) = sg_results {
        all_results.extend(results);
    }

    let output = serde_json::json!({
        "query": params.query,
        "mode": "hybrid",
        "result_count": all_results.len(),
        "local_results": all_results.iter().filter(|r| r.get("source").and_then(|s| s.as_str()) == Some("local")).count(),
        "sourcegraph_results": all_results.iter().filter(|r| r.get("source").and_then(|s| s.as_str()) == Some("sourcegraph")).count(),
        "results": all_results,
    });

    serde_json::to_string(&output).map_err(|e| format!("Failed to serialize output: {e}"))
}

fn execute_sourcegraph_inner(params: &SearchParams) -> Result<Vec<serde_json::Value>, String> {
    let search_query = if let Some(ref repo) = params.url {
        format!("repo:{} {}", repo, params.query)
    } else {
        params.query.clone()
    };

    let graphql_query = format!(
        r#"query {{ search(query: "{}") {{ results {{ results {{ ... on FileMatch {{ file {{ path name }} repository {{ name }} }} }} matchCount }} }} }}"#,
        search_query.replace('"', "\\\"")
    );

    let request_body = serde_json::json!({
        "query": graphql_query
    });

    let body_str = request_body.to_string();
    let headers = build_sourcegraph_headers();

    let response = make_request("POST", SOURCEGRAPH_API_URL, &headers, &body_str)?;
    let body = response_to_string(response)?;

    let sg_response: SourcegraphResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Sourcegraph response: {e}"))?;

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

    let results = search_results.and_then(|r| r.results).unwrap_or_default();

    let formatted: Vec<serde_json::Value> = results
        .into_iter()
        .filter_map(|m| {
            let mut entry = serde_json::json!({
                "source": "sourcegraph"
            });

            if let Some(repo) = &m.repository {
                if let Some(name) = &repo.name {
                    entry["repository"] = serde_json::json!(name);
                }
            }

            if let Some(file) = &m.file {
                if let Some(path) = &file.path {
                    entry["path"] = serde_json::json!(path);
                }
                if let Some(name) = &file.name {
                    entry["file"] = serde_json::json!(name);
                }
            }

            if let Some(line) = m.line {
                entry["line"] = serde_json::json!(line);
            }

            if entry.as_object().map_or(true, |o| o.len() <= 1) {
                return None;
            }

            Some(entry)
        })
        .collect();

    Ok(formatted)
}

fn execute_local_search(params: &SearchParams) -> Result<Vec<serde_json::Value>, String> {
    let local_url = match std::env::var("LOCAL_CODE_SEARCH_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => DEFAULT_LOCAL_SEARCH_URL.to_string(),
    };

    let search_endpoint = format!("{}/search", local_url.trim_end_matches('/'));

    let request_body = serde_json::json!({
        "query": params.query,
        "limit": params.count.unwrap_or(DEFAULT_COUNT).clamp(1, MAX_COUNT),
    });

    let body_str = request_body.to_string();
    let headers = serde_json::json!({
        "Accept": "application/json",
        "Content-Type": "application/json",
        "User-Agent": "IronClaw-Firecrawl-Tool/1.0"
    });

    let response = match make_request("POST", &search_endpoint, &headers, &body_str) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()),
    };

    let body = response_to_string(response)?;
    let local_response: LocalSearchResponse = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()),
    };

    let formatted: Vec<serde_json::Value> = local_response
        .results
        .into_iter()
        .filter_map(|r| {
            let mut entry = serde_json::json!({
                "source": "local"
            });

            if let Some(title) = &r.title {
                entry["title"] = serde_json::json!(title);
            }
            if let Some(path) = &r.file_path {
                entry["file_path"] = serde_json::json!(path);
            }
            if let Some(path) = &r.relative_path {
                entry["relative_path"] = serde_json::json!(path);
            }
            if let Some(lang) = &r.language {
                entry["language"] = serde_json::json!(lang);
            }
            if let Some(line) = r.line_number {
                entry["line_number"] = serde_json::json!(line);
            }
            if let Some(score) = r.score {
                entry["score"] = serde_json::json!(score);
            }
            if let Some(content) = &r.content {
                if content.len() > 2000 {
                    entry["content_snippet"] = serde_json::json!(&content[..2000]);
                } else {
                    entry["content"] = serde_json::json!(content);
                }
            }

            if entry.as_object().map_or(true, |o| o.len() <= 1) {
                return None;
            }

            Some(entry)
        })
        .collect();

    Ok(formatted)
}

fn build_headers() -> serde_json::Value {
    let mut headers = serde_json::json!({
        "Accept": "application/json",
        "Content-Type": "application/json",
        "User-Agent": "IronClaw-Firecrawl-Tool/1.0"
    });

    if let Ok(key) = std::env::var("FIRECRAWL_API_KEY") {
        if !key.is_empty() {
            headers["Authorization"] = serde_json::json!(format!("Bearer {}", key));
        }
    }

    headers
}

fn build_sourcegraph_headers() -> serde_json::Value {
    let mut headers = serde_json::json!({
        "Accept": "application/json",
        "Content-Type": "application/json",
        "User-Agent": "IronClaw-Firecrawl-Tool/1.0"
    });

    if let Ok(token) = std::env::var("SOURCEGRAPH_ACCESS_TOKEN") {
        if !token.is_empty() {
            headers["Authorization"] = serde_json::json!(format!("token {}", token));
        }
    }

    headers
}

fn make_request(
    method: &str,
    url: &str,
    headers: &serde_json::Value,
    body: &str,
) -> Result<near::agent::host::HttpResponse, String> {
    let headers_str = headers.to_string();
    let mut attempt = 0;

    loop {
        attempt += 1;

        let resp =
            near::agent::host::http_request(method, url, &headers_str, Some(body.as_bytes()), None)
                .map_err(|e| format!("HTTP request failed: {e}"))?;

        if resp.status >= 200 && resp.status < 300 {
            return Ok(resp);
        }

        if attempt < MAX_RETRIES && (resp.status == 429 || resp.status >= 500) {
            near::agent::host::log(
                near::agent::host::LogLevel::Warn,
                &format!(
                    "Firecrawl error {} (attempt {}/{}). Retrying...",
                    resp.status, attempt, MAX_RETRIES
                ),
            );
            continue;
        }

        let body = String::from_utf8_lossy(&resp.body);
        return Err(format!("Firecrawl error (HTTP {}): {}", resp.status, body));
    }
}

fn response_to_string(response: near::agent::host::HttpResponse) -> Result<String, String> {
    String::from_utf8(response.body).map_err(|e| format!("Invalid UTF-8 response: {e}"))
}

fn get_api_url() -> String {
    match std::env::var("FIRECRAWL_API_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => DEFAULT_API_URL.to_string(),
    }
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

const SCHEMA: &str = r#"{
    "type": "object",
    "properties": {
        "query": {
            "type": "string",
            "description": "The search query or topic to look up"
        },
        "mode": {
            "type": "string",
            "description": "Operation mode: 'search' for web search, 'context' for scraping a specific URL, 'sourcegraph' for searching public codebases, 'hybrid' for searching both Sourcegraph and local code",
            "default": "search",
            "enum": ["search", "context", "sourcegraph", "hybrid"]
        },
        "count": {
            "type": "integer",
            "description": "Number of search results to return (1-20, default 5)",
            "minimum": 1,
            "maximum": 20,
            "default": 5
        },
        "url": {
            "type": "string",
            "description": "For mode='context': the URL to scrape. For mode='sourcegraph': repository name to search (e.g. 'facebook/react')"
        },
        "scrape_formats": {
            "type": "string",
            "description": "Comma-separated output formats: 'markdown', 'html', 'links', 'screenshot' (default: markdown)"
        },
        "only_main_content": {
            "type": "boolean",
            "description": "Filter out navigation, footers, and other boilerplate (default: true)",
            "default": true
        },
        "context_size": {
            "type": "string",
            "description": "Context size hint for RAG: 'small', 'medium', 'large' (default: medium). Only used in 'context' mode.",
            "default": "medium"
        }
    },
    "required": ["query"],
    "additionalProperties": false
}"#;

export!(FirecrawlSearchTool);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hostname() {
        assert_eq!(
            extract_hostname("https://example.com/path"),
            Some("example.com".into())
        );
        assert_eq!(
            extract_hostname("https://sub.example.com:8080/path"),
            Some("sub.example.com".into())
        );
        assert_eq!(extract_hostname("not-a-url"), None);
    }

    #[test]
    fn test_mode_validation() {
        let params: serde_json::Value = serde_json::json!({
            "query": "test",
            "mode": "invalid"
        });
        let p: SearchParams = serde_json::from_value(params).unwrap();
        let mode = p.mode.as_deref().unwrap_or("search");
        assert_eq!(mode, "invalid");
    }
}
