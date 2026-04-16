//! HTTP service for the Firecrawl Search Service.

use crate::config::FirecrawlSearchConfig;
use crate::context::{execute_context, ContextRequest};
use crate::hybrid::{execute_hybrid, HybridRequest};
use crate::search::{execute_web_search, WebSearchRequest};
use crate::sourcegraph::{execute_sourcegraph, SourcegraphRequest};
use actix_web::{web, App, HttpResponse, HttpServer};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize, serde::Serialize)]
struct ErrorResponse {
    error: String,
}

pub struct SharedState {
    pub config: FirecrawlSearchConfig,
}

impl SharedState {
    pub fn new(config: FirecrawlSearchConfig) -> Self {
        Self { config }
    }
}

pub struct FirecrawlSearchService {
    state: Arc<SharedState>,
}

impl FirecrawlSearchService {
    pub fn new(config: FirecrawlSearchConfig) -> Self {
        Self {
            state: Arc::new(SharedState::new(config)),
        }
    }

    pub async fn run(self) -> std::io::Result<()> {
        let addr = format!(
            "{}:{}",
            self.state.config.service.bind_address, self.state.config.service.port
        );

        tracing::info!("Starting Firecrawl Search Service on {}", addr);

        let state = self.state.clone();
        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(state.clone()))
                .route("/search", web::post().to(search_handler))
                .route("/context", web::post().to(context_handler))
                .route("/sourcegraph", web::post().to(sourcegraph_handler))
                .route("/hybrid", web::post().to(hybrid_handler))
                .route("/health", web::get().to(health_handler))
        })
        .bind(&addr)?
        .run()
        .await
    }
}

async fn search_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<WebSearchRequest>,
) -> HttpResponse {
    match execute_web_search(&state.config.firecrawl, &body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse { error: e }),
    }
}

async fn context_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<ContextRequest>,
) -> HttpResponse {
    match execute_context(&state.config.firecrawl, &body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse { error: e }),
    }
}

async fn sourcegraph_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<SourcegraphRequest>,
) -> HttpResponse {
    match execute_sourcegraph(&state.config.sourcegraph, &body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse { error: e }),
    }
}

async fn hybrid_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<HybridRequest>,
) -> HttpResponse {
    match execute_hybrid(&state.config, &body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse { error: e }),
    }
}

async fn health_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "firecrawl-search-service",
    }))
}
