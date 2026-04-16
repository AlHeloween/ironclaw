//! HTTP service and file watcher for the Local Code Search Service.

use crate::config::{IndexConfig, LocalCodeSearchConfig};
use crate::index::{CodeIndex, SymbolInfo};
use crate::search::{SearchQuery, SearchResponse, SearchService};
use crate::symbols::{detect_language, extract_symbols, should_index_language};
use actix_web::{web, App, HttpResponse, HttpServer};
use md5::{Digest, Md5};
use notify::{Event, EventKind, RecommendedWatcher, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tantivy::doc;
use walkdir::WalkDir;

pub struct SharedState {
    pub indexes: RwLock<HashMap<String, CodeIndex>>,
    pub config: LocalCodeSearchConfig,
    pub indexing_progress: RwLock<IndexingProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexingProgress {
    pub indexing: bool,
    pub current_index: Option<String>,
    pub files_indexed: u64,
    pub total_files_estimated: u64,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl SharedState {
    pub fn new(config: LocalCodeSearchConfig) -> Self {
        Self {
            indexes: RwLock::new(HashMap::new()),
            config,
            indexing_progress: RwLock::new(IndexingProgress::default()),
        }
    }
}

pub struct CodeSearchService {
    state: Arc<SharedState>,
}

impl CodeSearchService {
    pub fn new(config: LocalCodeSearchConfig) -> Self {
        Self {
            state: Arc::new(SharedState::new(config)),
        }
    }

    pub async fn run(self) -> std::io::Result<()> {
        let state = self.state.clone();
        state.initialize_indexes().await;

        if state.config.service.watch_enabled {
            let watch_state = self.state.clone();
            tokio::spawn(async move {
                watch_state.run_watcher().await;
            });
        }

        let addr = format!(
            "{}:{}",
            self.state.config.service.bind_address, self.state.config.service.port
        );
        let state = self.state.clone();

        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(state.clone()))
                .route("/search", web::post().to(search_handler))
                .route("/symbol-search", web::post().to(symbol_search_handler))
                .route("/index", web::post().to(index_handler))
                .route("/index/rebuild", web::post().to(rebuild_handler))
                .route("/index/stats", web::get().to(stats_handler))
                .route("/status", web::get().to(status_handler))
                .route("/health", web::get().to(health_handler))
        })
        .bind(&addr)?
        .run()
        .await
    }
}

impl SharedState {
    async fn initialize_indexes(&self) {
        let index_dir = self.index_dir();
        std::fs::create_dir_all(&index_dir).ok();

        for index_config in &self.config.indexes {
            if !index_config.enabled {
                continue;
            }
            match CodeIndex::open_or_create(&index_config.name, &index_dir) {
                Ok(index) => {
                    self.indexes.write().unwrap().insert(index_config.name.clone(), index);
                    self.index_path(&index_config).await;
                }
                Err(e) => {
                    tracing::error!("Failed to initialize index '{}': {}", index_config.name, e);
                }
            }
        }
    }

    fn index_dir(&self) -> PathBuf {
        dirs::home_dir()
            .map(|mut p| {
                p.push(".ironclaw/code-search-indexes");
                p
            })
            .expect("Could not determine home directory")
    }

    fn should_exclude(&self, path: &Path, index_config: &IndexConfig) -> bool {
        let path_str = path.to_string_lossy();
        for pattern in &index_config.exclude {
            if path_str.contains(pattern) {
                return true;
            }
        }
        false
    }

    fn should_index(&self, path: &Path, index_config: &IndexConfig) -> bool {
        if path.is_dir() {
            return !self.should_exclude(path, index_config);
        }

        if !self.should_exclude(path, index_config) {
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(u64::MAX);
            if size > index_config.max_file_size {
                return false;
            }

            let language = detect_language(&path.to_string_lossy());
            return should_index_language(&index_config.languages, language);
        }
        false
    }

    async fn index_path(&self, index_config: &IndexConfig) {
        let root = PathBuf::from(&index_config.path);
        if !root.exists() {
            tracing::warn!("Index path does not exist: {:?}", root);
            return;
        }

        {
            let mut progress = self.indexing_progress.write().unwrap();
            progress.indexing = true;
            progress.current_index = Some(index_config.name.clone());
            progress.started_at = Some(chrono::Utc::now().to_rfc3339());
        }

        let mut indexes = self.indexes.write().unwrap();
        let index = match indexes.get_mut(&index_config.name) {
            Some(i) => i,
            None => return,
        };

        let title_field = index.title_field();
        let content_field = index.content_field();
        let file_path_field = index.file_path_field();
        let relative_path_field = index.relative_path_field();
        let index_name_field = index.index_name_field();
        let language_field = index.language_field();
        let line_number_field = index.line_number_field();
        let content_hash_field = index.content_hash_field();
        let indexed_at_field = index.indexed_at_field();
        let symbols_field = index.symbols_field();

        let mut count = 0u64;
        let mut batch_size = 0u64;
        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_entry(|e| self.should_index(e.path(), index_config))
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            if !entry.file_type().is_file() {
                continue;
            }

            let content = match std::fs::read_to_string(entry.path()) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut hasher = Md5::new();
            hasher.update(content.as_bytes());
            let hash = format!("{:x}", hasher.finalize());
            let language = detect_language(&entry.path().to_string_lossy());

            let symbols: Vec<SymbolInfo> = if index_config.symbols_enabled {
                extract_symbols(&content, language)
            } else {
                Vec::new()
            };

            let symbols_json = serde_json::to_string(&symbols).unwrap_or_default();
            let relative = entry
                .path()
                .strip_prefix(&root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .to_string();

            index.writer().add_document(doc!(
                title_field => entry.file_name().to_string_lossy().to_string(),
                content_field => content,
                file_path_field => entry.path().to_string_lossy().to_string(),
                relative_path_field => relative,
                index_name_field => index_config.name.clone(),
                language_field => language.to_string(),
                line_number_field => 0u64,
                content_hash_field => hash,
                indexed_at_field => tantivy::DateTime::from_timestamp_secs(chrono::Utc::now().timestamp()),
                symbols_field => symbols_json,
            )).ok();

            count += 1;
            batch_size += 1;
            if batch_size % 100 == 0 {
                let mut progress = self.indexing_progress.write().unwrap();
                progress.files_indexed = count;
                tracing::info!("Indexed {} files for index '{}'...", count, index_config.name);
                batch_size = 0;
            }
        }

        index.commit().ok();

        {
            let mut progress = self.indexing_progress.write().unwrap();
            progress.files_indexed = count;
            progress.indexing = false;
            progress.completed_at = Some(chrono::Utc::now().to_rfc3339());
        }

        tracing::info!(
            "Indexed {} files for index '{}'",
            count,
            index_config.name
        );
    }

    async fn run_watcher(&self) {
        let (tx, rx) = std::sync::mpsc::channel();
        let debounce_ms = self.config.service.watch_debounce_ms;

        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    tx.send(event).ok();
                }
            },
            notify::Config::default(),
        )
        .ok();

        if let Some(mut watcher) = watcher {
            for index_config in &self.config.indexes {
                if !index_config.enabled {
                    continue;
                }
                let path = PathBuf::from(&index_config.path);
                if path.exists() {
                    watcher
                        .watch(&path, notify::RecursiveMode::Recursive)
                        .ok();
                }
            }

            loop {
                match rx.recv_timeout(Duration::from_millis(debounce_ms)) {
                    Ok(event) => {
                        match event.kind {
                            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                                for path in event.paths {
                                    self.handle_file_change(&path).await;
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(_) => {}
                }
            }
        }
    }

    async fn handle_file_change(&self, path: &Path) {
        let mut indexes = self.indexes.write().unwrap();
        for index_config in &self.config.indexes {
            if !index_config.enabled {
                continue;
            }
            let root = PathBuf::from(&index_config.path);
            if !path.starts_with(&root) {
                continue;
            }

            if let Some(index) = indexes.get_mut(&index_config.name) {
                if path.is_file() && self.should_index(path, index_config) {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        let mut hasher = Md5::new();
                        hasher.update(content.as_bytes());
                        let hash = format!("{:x}", hasher.finalize());
                        let language = detect_language(&path.to_string_lossy());
                        let symbols: Vec<SymbolInfo> = if index_config.symbols_enabled {
                            extract_symbols(&content, language)
                        } else {
                            Vec::new()
                        };
                        let symbols_json = serde_json::to_string(&symbols).unwrap_or_default();
                        let relative = path
                            .strip_prefix(&root)
                            .unwrap_or(path)
                            .to_string_lossy()
                            .to_string();

                        index.delete_by_hash(&hash).ok();

                        let title_field = index.title_field();
                        let content_field = index.content_field();
                        let file_path_field = index.file_path_field();
                        let relative_path_field = index.relative_path_field();
                        let index_name_field = index.index_name_field();
                        let language_field = index.language_field();
                        let line_number_field = index.line_number_field();
                        let content_hash_field = index.content_hash_field();
                        let indexed_at_field = index.indexed_at_field();
                        let symbols_field = index.symbols_field();

            index.writer().add_document(doc!(
                            title_field => path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                            content_field => content,
                            file_path_field => path.to_string_lossy().to_string(),
                            relative_path_field => relative,
                            index_name_field => index_config.name.clone(),
                            language_field => language.to_string(),
                            line_number_field => 0u64,
                            content_hash_field => hash,
                            indexed_at_field => tantivy::DateTime::from_timestamp_secs(chrono::Utc::now().timestamp()),
                            symbols_field => symbols_json,
                        )).ok();

                        index.commit().ok();
                    }
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct IndexRequest {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct IndexResponse {
    pub success: bool,
    pub message: String,
}

async fn search_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<SearchQuery>,
) -> HttpResponse {
    let indexes = state.indexes.read().unwrap();

    let results: Vec<_> = if let Some(index_name) = &body.index {
        indexes
            .get(index_name)
            .map(|index| {
                SearchService::search(&index.index, &body.query, body.limit)
            })
            .unwrap_or_default()
    } else {
        indexes
            .values()
            .flat_map(|index| {
                SearchService::search(&index.index, &body.query, body.limit / indexes.len().max(1))
            })
            .collect()
    };

    HttpResponse::Ok().json(SearchResponse {
        total: results.len(),
        query: body.query.clone(),
        results,
    })
}

async fn symbol_search_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<SearchQuery>,
) -> HttpResponse {
    let indexes = state.indexes.read().unwrap();

    let results: Vec<_> = if let Some(index_name) = &body.index {
        indexes
            .get(index_name)
            .map(|index| SearchService::symbol_search(&index.index, &body.query, body.limit))
            .unwrap_or_default()
    } else {
        indexes
            .values()
            .flat_map(|index| {
                SearchService::symbol_search(&index.index, &body.query, body.limit / indexes.len().max(1))
            })
            .collect()
    };

    HttpResponse::Ok().json(SearchResponse {
        total: results.len(),
        query: body.query.clone(),
        results,
    })
}

async fn index_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<IndexRequest>,
) -> HttpResponse {
    let index_dir = state.index_dir();
    std::fs::create_dir_all(&index_dir).ok();

    match CodeIndex::open_or_create(&body.name, &index_dir) {
        Ok(index) => {
            state.indexes.write().unwrap().insert(body.name.clone(), index);
            let config = IndexConfig {
                name: body.name.clone(),
                path: body.path.clone(),
                languages: vec!["all".to_string()],
                include: vec![],
                exclude: vec![],
                symbols_enabled: true,
                max_file_size: 1024 * 1024,
                enabled: true,
            };
            state.index_path(&config).await;
            HttpResponse::Ok().json(IndexResponse {
                success: true,
                message: format!("Index '{}' created and populated", body.name),
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(IndexResponse {
            success: false,
            message: e.to_string(),
        }),
    }
}

async fn rebuild_handler(
    state: web::Data<Arc<SharedState>>,
    body: web::Json<IndexRequest>,
) -> HttpResponse {
    let mut indexes = state.indexes.write().unwrap();
    if indexes.remove(&body.name).is_some() {
        let index_dir = state.index_dir().join(&body.name);
        if index_dir.exists() {
            std::fs::remove_dir_all(&index_dir).ok();
        }

        match CodeIndex::open_or_create(&body.name, &state.index_dir()) {
            Ok(new_index) => {
                indexes.insert(body.name.clone(), new_index);
                let config = IndexConfig {
                    name: body.name.clone(),
                    path: body.path.clone(),
                    languages: vec!["all".to_string()],
                    include: vec![],
                    exclude: vec![],
                    symbols_enabled: true,
                    max_file_size: 1024 * 1024,
                    enabled: true,
                };
                state.index_path(&config).await;
                return HttpResponse::Ok().json(IndexResponse {
                    success: true,
                    message: format!("Index '{}' rebuilt", body.name),
                });
            }
            Err(e) => {
                return HttpResponse::InternalServerError().json(IndexResponse {
                    success: false,
                    message: e.to_string(),
                });
            }
        }
    }

    HttpResponse::NotFound().json(IndexResponse {
        success: false,
        message: format!("Index '{}' not found", body.name),
    })
}

async fn stats_handler(state: web::Data<Arc<SharedState>>) -> HttpResponse {
    let indexes = state.indexes.read().unwrap();
    let stats: Vec<_> = indexes.values().map(|i| i.stats()).collect();
    HttpResponse::Ok().json(serde_json::json!({
        "indexes": stats,
        "total_indexes": stats.len(),
    }))
}

async fn status_handler(state: web::Data<Arc<SharedState>>) -> HttpResponse {
    let progress = state.indexing_progress.read().unwrap();
    let indexes = state.indexes.read().unwrap();
    let stats: Vec<_> = indexes.values().map(|i| i.stats()).collect();
    HttpResponse::Ok().json(serde_json::json!({
        "service": "local-code-search",
        "version": env!("CARGO_PKG_VERSION"),
        "indexing": *progress,
        "indexes": stats,
        "total_indexes": stats.len(),
    }))
}

async fn health_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "local-code-search",
    }))
}
