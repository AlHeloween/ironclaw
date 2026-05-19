//! HTTP service for the universal search service.

use crate::config::{default_agent_system_prompt, Config};
use crate::hybrid::{execute_hybrid, HybridRequest};
use crate::web::{execute_context, execute_web_search, execute_sourcegraph, ContextRequest, SourcegraphRequest, WebSearchRequest};
use actix_web::{web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, Semaphore};

static TOOL_DEFINITIONS: OnceLock<Vec<serde_json::Value>> = OnceLock::new();

fn get_tool_definitions() -> &'static [serde_json::Value] {
    TOOL_DEFINITIONS.get_or_init(|| {
        vec![
            serde_json::json!({
                "name": "search",
                "description": "Search the web for a query and return scraped results with markdown content",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        }
                    },
                    "required": ["query"]
                }
            }),
            serde_json::json!({
                "name": "scrape",
                "description": "Extract content from a specific URL. Supports HTML, PDF, and documents. Returns markdown.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to scrape"
                        }
                    },
                    "required": ["url"]
                }
            }),
            serde_json::json!({
                "name": "crawl",
                "description": "Crawl multiple pages from a seed URL",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "Seed URL to start crawling from"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum number of pages to crawl",
                            "default": 10
                        }
                    },
                    "required": ["url"]
                }
            }),
            serde_json::json!({
                "name": "map",
                "description": "Discover all URLs on a domain, optionally filtered by keyword",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "Domain URL to map"
                        },
                        "search": {
                            "type": "string",
                            "description": "Optional keyword to filter URLs"
                        }
                    },
                    "required": ["url"]
                }
            }),
        ]
    })
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    service: String,
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    status: String,
    service: String,
    web_search: ServiceStatus,
    agent: AgentStatus,
}

#[derive(Debug, Serialize)]
struct ServiceStatus {
    enabled: bool,
    port: u16,
    healthy: bool,
}

#[derive(Debug, Serialize)]
struct AgentStatus {
    enabled: bool,
    healthy: bool,
}

#[derive(Debug, Clone)]
pub enum AgentJobStatus {
    Pending,
    Processing { current_turn: u32, last_tool: Option<String>, last_reasoning: Option<String> },
    Completed(AgentResponse),
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct AgentJob {
    #[allow(dead_code)]
    pub id: String,
    pub status: AgentJobStatus,
    pub created_at: u64,
    pub cancel_flag: Arc<AtomicBool>,
}

pub struct SharedState {
    pub config: Config,
    pub agent_jobs: Arc<RwLock<HashMap<String, AgentJob>>>,
    pub last_agent_request_ts: Arc<AtomicU64>,
    pub rate_limit_secs: u64,
    pub concurrency_semaphore: Arc<Semaphore>,
    pub client: reqwest::Client,
    pub http_client: reqwest::Client,
}

impl SharedState {
    pub fn new(config: Config) -> Self {
        let max_concurrent = config.agent.max_concurrent as usize;
        let rate_limit_secs = config.agent.rate_limit_seconds as u64;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build LLM client");
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self {
            config,
            agent_jobs: Arc::new(RwLock::new(HashMap::new())),
            last_agent_request_ts: Arc::new(AtomicU64::new(0)),
            rate_limit_secs,
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            client,
            http_client,
        }
    }
}

pub struct SearchHttpService {
    state: Arc<SharedState>,
}

impl SearchHttpService {
    pub fn new(config: Config) -> Self {
        Self {
            state: Arc::new(SharedState::new(config)),
        }
    }

    pub async fn run(self) -> std::io::Result<()> {
        let addr = format!("{}:{}", self.state.config.service.bind_address, self.state.config.service.port);
        tracing::info!("Starting Universal Search Service on {}", addr);

        let state = self.state.clone();
        let cleanup_state = state.clone();
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);

        let cleanup_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {
                        let ttl_secs = cleanup_state.config.agent.job_ttl_seconds as u64;
                        let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                        let mut jobs = cleanup_state.agent_jobs.write().unwrap();
                        jobs.retain(|_, job| now_ts.saturating_sub(job.created_at) < ttl_secs);
                    }
                }
            }
        });

        let state = self.state.clone();
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(state.clone()))
                .route("/health", web::get().to(health_handler))
                .route("/status", web::get().to(status_handler))
                .route("/agent", web::post().to(agent_handler))
                .route("/agent/{id}", web::get().to(agent_status_handler))
                .route("/agent/{id}", web::delete().to(agent_cancel_handler))
                .route("/web/search", web::post().to(web_search_handler))
                .route("/web/context", web::post().to(web_context_handler))
                .route("/web/sourcegraph", web::post().to(sourcegraph_handler))
                .route("/hybrid", web::post().to(hybrid_handler))
        })
        .bind(&addr)?
        .run()
        .await;

    let _ = shutdown_tx.send(());
    let _ = cleanup_handle.await;
    server
    }
}

async fn health_handler() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        service: "universal-search-service".to_string(),
    })
}

async fn status_handler(state: web::Data<Arc<SharedState>>) -> HttpResponse {
    HttpResponse::Ok().json(StatusResponse {
        status: "healthy".to_string(),
        service: "universal-search-service".to_string(),
        web_search: ServiceStatus {
            enabled: true,
            port: state.config.service.port,
            healthy: true,
        },
        agent: AgentStatus {
            enabled: state.config.agent.enabled,
            healthy: state.config.agent.enabled,
        },
    })
}

#[derive(Debug, Deserialize)]
pub struct AgentRequest {
    pub query: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub urls: Option<Vec<String>>,
    #[serde(default)]
    pub max_turns: Option<u32>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub query: String,
    pub answer: String,
    pub turns: u32,
    pub tool_calls: Vec<ToolCall>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub turn: u32,
    pub tool: Option<String>,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
}

async fn agent_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<AgentRequest>,
) -> HttpResponse {
    if !state.config.agent.enabled {
        return HttpResponse::ServiceUnavailable().json(ErrorResponse {
            error: "Agent endpoint is disabled".to_string(),
        });
    }

    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let rate_limit = state.rate_limit_secs;

    loop {
        let last_ts = state.last_agent_request_ts.load(Ordering::Relaxed);
        if now_ts.saturating_sub(last_ts) < rate_limit {
            return HttpResponse::TooManyRequests().json(ErrorResponse {
                error: format!(
                    "Rate limited. Please wait {} seconds between requests.",
                    rate_limit
                ),
            });
        }
        if state.last_agent_request_ts.compare_exchange(
            last_ts, now_ts, Ordering::Relaxed, Ordering::Relaxed,
        ).is_ok() {
            break;
        }
    }

    match state.concurrency_semaphore.clone().acquire_owned().await {
        Ok(_permit) => {}
        Err(_) => {
            return HttpResponse::TooManyRequests().json(ErrorResponse {
                error: format!("Maximum {} concurrent agent jobs reached", state.config.agent.max_concurrent),
            });
        }
    }

    let job_id = uuid::Uuid::new_v4().to_string();
    let job = AgentJob {
        id: job_id.clone(),
        status: AgentJobStatus::Pending,
        created_at: now_ts,
        cancel_flag: Arc::new(AtomicBool::new(false)),
    };

    {
        let mut jobs = state.agent_jobs.write().unwrap();
        jobs.insert(job_id.clone(), job);
    }

    let state_clone = state.get_ref().clone();
    let request = body.into_inner();
    let job_id_clone = job_id.clone();

    tokio::spawn(async move {
        run_agent_loop(&state_clone, &job_id_clone, request).await;
    });

    HttpResponse::Accepted().json(serde_json::json!({
        "success": true,
        "id": job_id,
        "status": "processing"
    }))
}

async fn agent_status_handler(
    state: web::Data<Arc<SharedState>>,
    path: web::Path<String>,
) -> HttpResponse {
    let job_id = path.into_inner();
    let jobs = state.agent_jobs.read().unwrap();

    match jobs.get(&job_id) {
        Some(job) => match &job.status {
            AgentJobStatus::Pending => HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "status": "processing"
            })),
            AgentJobStatus::Processing { current_turn, last_tool, last_reasoning } => HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "status": "processing",
                "current_turn": current_turn,
                "last_tool": last_tool,
                "last_reasoning": last_reasoning
            })),
            AgentJobStatus::Completed(response) => HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "status": "completed",
                "data": response
            })),
            AgentJobStatus::Failed(err) => HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "status": "failed",
                "error": err
            })),
            AgentJobStatus::Cancelled => HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "status": "cancelled"
            })),
        },
        None => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Job {} not found", job_id),
        }),
    }
}

async fn agent_cancel_handler(
    state: web::Data<Arc<SharedState>>,
    path: web::Path<String>,
) -> HttpResponse {
    let job_id = path.into_inner();
    let mut jobs = state.agent_jobs.write().unwrap();

    match jobs.get_mut(&job_id) {
        Some(job) => {
            job.cancel_flag.store(true, Ordering::SeqCst);
            job.status = AgentJobStatus::Cancelled;
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "status": "cancelled"
            }))
        }
        None => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Job {} not found", job_id),
        }),
    }
}

async fn run_agent_loop(state: &SharedState, job_id: &str, request: AgentRequest) {
    let config = &state.config.agent;
    let max_turns = request.max_turns.unwrap_or(config.max_turns);
    let model = request.model.as_deref().unwrap_or(&config.model);
    let default_prompt = default_agent_system_prompt();
    let system_prompt = request.system_prompt.as_deref()
        .or(config.system_prompt.as_deref())
        .unwrap_or(default_prompt.as_str());

    let mut messages: Vec<serde_json::Value> = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    messages.push(serde_json::json!({
        "role": "user",
        "content": request.query
    }));

    let tools = get_tool_definitions();

    let mut accumulated_text = String::new();

    for turn in 1..=max_turns {
        // Rate-limit: delay between turns to avoid rushing the Claude API
        if turn > 1 {
            let delay_ms = config.turn_delay_ms;
            tracing::debug!("Agent job {}: turn delay {}ms before turn {}", job_id, delay_ms, turn);
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }

        {
            let mut jobs = state.agent_jobs.write().unwrap();
            if let Some(job) = jobs.get_mut(job_id) {
                if job.cancel_flag.load(Ordering::SeqCst) {
                    return;
                }
                job.status = AgentJobStatus::Processing {
                    current_turn: turn,
                    last_tool: None,
                    last_reasoning: None,
                };
            }
        }

        let response = match call_claude(state, model, system_prompt, &messages, &tools).await {
            Ok(resp) => resp,
            Err(e) => {
                let mut jobs = state.agent_jobs.write().unwrap();
                if let Some(job) = jobs.get_mut(job_id) {
                    job.status = AgentJobStatus::Failed(format!("Claude API error: {}", e));
                }
                return;
            }
        };

        let content_blocks = response
            .get("content")
            .and_then(|c| c.as_array())
            .map(|a| a.to_vec())
            .unwrap_or_default();

        let mut text_blocks: Vec<serde_json::Value> = Vec::new();
        let mut turn_text = String::new();

        for block in &content_blocks {
            let block_type = block.get("type").and_then(|t| t.as_str());
            if block_type == Some("text") {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    if !turn_text.is_empty() {
                        turn_text.push('\n');
                    }
                    turn_text.push_str(text);
                    text_blocks.push(block.clone());
                }
            }
        }

        let tool_blocks: Vec<&serde_json::Value> = content_blocks.iter()
            .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
            .collect();

        let has_tool_use = !tool_blocks.is_empty();

        if !turn_text.is_empty() {
            accumulated_text.push_str(&turn_text);
            accumulated_text.push('\n');
        }

        if has_tool_use {
            let mut assistant_content: Vec<serde_json::Value> = Vec::new();

            for text_block in &text_blocks {
                assistant_content.push(text_block.clone());
            }
            for block in &tool_blocks {
                assistant_content.push((*block).clone());
            }

            let mut tool_results: Vec<serde_json::Value> = Vec::new();

            let mut first_tool = true;
            for block in &tool_blocks {
                // Rate-limit: delay between sequential tool calls
                if !first_tool {
                    tokio::time::sleep(std::time::Duration::from_millis(config.turn_delay_ms)).await;
                }
                first_tool = false;

                let tool_name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let input = block.get("input").cloned().unwrap_or(serde_json::json!({}));
                let tool_id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");

                {
                    let mut jobs = state.agent_jobs.write().unwrap();
                    if let Some(job) = jobs.get_mut(job_id) {
                        job.status = AgentJobStatus::Processing {
                            current_turn: turn,
                            last_tool: Some(tool_name.to_string()),
                            last_reasoning: if !turn_text.is_empty() { Some(turn_text.clone()) } else { None },
                        };
                    }
                }

                let output = execute_tool(state, tool_name, &input).await;

                tool_calls.push(ToolCall {
                    turn,
                    tool: Some(tool_name.to_string()),
                    input: input.clone(),
                    output: output.clone(),
                });

                tool_results.push(serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": tool_id,
                    "content": output.to_string()
                }));
            }

            messages.push(serde_json::json!({
                "role": "assistant",
                "content": assistant_content
            }));

            messages.push(serde_json::json!({
                "role": "user",
                "content": tool_results
            }));

            if turn == max_turns && !accumulated_text.is_empty() {
                let answer = accumulated_text.trim().to_string();
                tool_calls.push(ToolCall {
                    turn,
                    tool: None,
                    input: serde_json::json!(null),
                    output: serde_json::json!({"answer": &answer}),
                });
                let resp = AgentResponse {
                    query: request.query.clone(),
                    answer,
                    turns: turn,
                    tool_calls,
                    model: model.to_string(),
                };
                let mut jobs = state.agent_jobs.write().unwrap();
                if let Some(job) = jobs.get_mut(job_id) {
                    job.status = AgentJobStatus::Completed(resp);
                }
                return;
            }
        } else {
            let answer = if turn_text.is_empty() {
                "No response from Claude.".to_string()
            } else {
                turn_text
            };

            tool_calls.push(ToolCall {
                turn,
                tool: None,
                input: serde_json::json!(null),
                output: serde_json::json!({"answer": &answer}),
            });

            let resp = AgentResponse {
                query: request.query.clone(),
                answer,
                turns: turn,
                tool_calls,
                model: model.to_string(),
            };

            let mut jobs = state.agent_jobs.write().unwrap();
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = AgentJobStatus::Completed(resp);
            }
            return;
        }
    }

    let partial_answer = if accumulated_text.is_empty() {
        "Reached maximum turns without a conclusive answer.".to_string()
    } else {
        accumulated_text.trim().to_string()
    };
    let response = AgentResponse {
        query: request.query.clone(),
        answer: partial_answer,
        turns: max_turns,
        tool_calls,
        model: model.to_string(),
    };

    let mut jobs = state.agent_jobs.write().unwrap();
    if let Some(job) = jobs.get_mut(job_id) {
        job.status = AgentJobStatus::Completed(response);
    }
}

async fn call_claude(
    state: &SharedState,
    model: &str,
    system_prompt: &str,
    messages: &[serde_json::Value],
    tools: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    let config = &state.config.agent;
    // Prefer config values, fall back to env vars
    let anthropic_key = config.anthropic_api_key.clone()
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        .ok_or("ANTHROPIC_API_KEY not configured")?;
    let mut anthropic_base = config.anthropic_base_url.clone()
        .or_else(|| std::env::var("ANTHROPIC_BASE_URL").ok())
        .unwrap_or_else(|| "https://api.anthropic.com".to_string());
    // Strip trailing /v1 or / since the SDK adds /v1/messages
    anthropic_base = anthropic_base.trim_end_matches("/v1").trim_end_matches('/').to_string();

    let url = format!("{}/v1/messages", anthropic_base);

    let request_body = serde_json::json!({
        "model": model,
        "max_tokens": config.max_output_tokens,
        "system": system_prompt,
        "messages": messages,
        "tools": tools,
        "tool_choice": {"type": "auto"}
    });

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("x-api-key", reqwest::header::HeaderValue::from_str(&anthropic_key)
        .map_err(|e| format!("Invalid API key header: {}", e))?);
    headers.insert("anthropic-version", reqwest::header::HeaderValue::from_static("2023-06-01"));
    headers.insert("content-type", reqwest::header::HeaderValue::from_static("application/json"));

    for attempt in 0..config.retry_max_attempts {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(config.retry_delay_seconds)).await;
        }

        let resp = state.client
            .post(&url)
            .headers(headers.clone())
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status = resp.status();
        if status.is_success() {
            let body: serde_json::Value = resp.json().await
                .map_err(|e| format!("Failed to parse response: {}", e))?;
            tracing::info!("Claude API response: {}", serde_json::to_string(&body).unwrap_or_default());
            return Ok(body);
        }

        if status == 429 || status.is_server_error() {
            tracing::warn!("Claude API error {} (attempt {}/{}). Retrying...", status, attempt + 1, config.retry_max_attempts);
            continue;
        }

        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Claude API error (HTTP {}): {}", status, body));
    }

    Err(format!("Max retries ({}) exceeded", config.retry_max_attempts))
}

async fn execute_tool(state: &SharedState, tool_name: &str, input: &serde_json::Value) -> serde_json::Value {
    match tool_name {
        "search" => {
            let query = input.get("query").and_then(|q| q.as_str()).unwrap_or("");
            let web_request = WebSearchRequest {
                query: query.to_string(),
                count: Some(5),
                scrape_formats: None,
                only_main_content: Some(true),
            };
            match execute_web_search(&state.http_client, &state.config.web_search.firecrawl, &web_request).await {
                Ok(resp) => serde_json::to_value(&resp).unwrap_or(serde_json::json!({"error": "Failed to serialize response"})),
                Err(e) => serde_json::json!({"error": e}),
            }
        }
        "scrape" => {
            let url = input.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let context_request = ContextRequest {
                query: url.to_string(),
                url: url.to_string(),
                scrape_formats: Some("markdown".to_string()),
                only_main_content: Some(true),
            };
            match execute_context(&state.http_client, &state.config.web_search.firecrawl, &context_request).await {
                Ok(resp) => serde_json::to_value(&resp).unwrap_or(serde_json::json!({"error": "Failed to serialize response"})),
                Err(e) => serde_json::json!({"error": e}),
            }
        }
        "crawl" => {
            let url = input.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let limit = input.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as u32;
            serde_json::json!({
                "status": "started",
                "url": url,
                "limit": limit,
                "note": "Crawl is asynchronous. Use the Firecrawl API directly for status tracking."
            })
        }
        "map" => {
            let url = input.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let search = input.get("search").and_then(|s| s.as_str()).map(|s| s.to_string());
            serde_json::json!({
                "status": "not_implemented",
                "url": url,
                "search": search,
                "note": "Map tool not yet implemented in local Firecrawl client."
            })
        }
        _ => serde_json::json!({"error": format!("Unknown tool: {}", tool_name)}),
    }
}

async fn web_search_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<WebSearchRequest>,
) -> HttpResponse {
    match execute_web_search(&state.http_client, &state.config.web_search.firecrawl, &body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse { error: e }),
    }
}

async fn web_context_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<ContextRequest>,
) -> HttpResponse {
    match execute_context(&state.http_client, &state.config.web_search.firecrawl, &body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse { error: e }),
    }
}

async fn sourcegraph_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<SourcegraphRequest>,
) -> HttpResponse {
    match execute_sourcegraph(&state.http_client, &state.config.web_search.sourcegraph, &body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse { error: e }),
    }
}

async fn hybrid_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<HybridRequest>,
) -> HttpResponse {
    match execute_hybrid(&state.http_client, &state.config, &body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse { error: e }),
    }
}
