//! Search query and result types.

use crate::index::SymbolInfo;
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    #[serde(default)]
    pub index: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub symbols_only: bool,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub content: String,
    pub file_path: String,
    pub relative_path: String,
    pub index_name: String,
    pub language: String,
    pub line_number: u32,
    pub score: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: usize,
    pub query: String,
}

pub struct SearchService;

impl SearchService {
    pub fn search(index: &tantivy::Index, query_text: &str, limit: usize) -> Vec<SearchResult> {
        let schema = index.schema();
        let title_field = schema.get_field("title").unwrap();
        let content_field = schema.get_field("content").unwrap();

        let query_parser = QueryParser::for_index(index, vec![title_field, content_field]);
        let query = match query_parser.parse_query(query_text) {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .ok();
        let reader = match reader {
            Some(r) => r,
            None => return Vec::new(),
        };

        let file_path_field = schema.get_field("file_path").unwrap();
        let relative_path_field = schema.get_field("relative_path").unwrap();
        let index_name_field = schema.get_field("index_name").unwrap();
        let language_field = schema.get_field("language").unwrap();
        let line_number_field = schema.get_field("line_number").unwrap();
        let symbols_field = schema.get_field("symbols").unwrap();

        let searcher = reader.searcher();
        let top_docs = match searcher.search(&query, &TopDocs::with_limit(limit)) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        top_docs
            .into_iter()
            .map(|(score, doc_address)| {
                let doc: tantivy::schema::TantivyDocument = searcher.doc(doc_address).unwrap();
                let title: String = doc
                    .get_first(title_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let content: String = doc
                    .get_first(content_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let file_path: String = doc
                    .get_first(file_path_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let relative_path: String = doc
                    .get_first(relative_path_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let index_name: String = doc
                    .get_first(index_name_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let language: String = doc
                    .get_first(language_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let line_number: u32 = doc
                    .get_first(line_number_field)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;

                let symbols: Vec<SymbolInfo> = doc
                    .get_first(symbols_field)
                    .and_then(|v| v.as_str())
                    .map(|s| serde_json::from_str(s).unwrap_or_default())
                    .unwrap_or_default();

                SearchResult {
                    title,
                    content,
                    file_path,
                    relative_path,
                    index_name,
                    language,
                    line_number,
                    score,
                    symbols,
                }
            })
            .collect()
    }

    pub fn symbol_search(
        index: &tantivy::Index,
        symbol_name: &str,
        limit: usize,
    ) -> Vec<SearchResult> {
        let schema = index.schema();
        let symbols_field = schema.get_field("symbols").unwrap();
        let title_field = schema.get_field("title").unwrap();
        let content_field = schema.get_field("content").unwrap();
        let file_path_field = schema.get_field("file_path").unwrap();
        let relative_path_field = schema.get_field("relative_path").unwrap();
        let index_name_field = schema.get_field("index_name").unwrap();
        let language_field = schema.get_field("language").unwrap();
        let line_number_field = schema.get_field("line_number").unwrap();

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .ok();
        let reader = match reader {
            Some(r) => r,
            None => return Vec::new(),
        };

        let mut results = Vec::new();
        let searcher = reader.searcher();
        let all_docs =
            match searcher.search(&tantivy::query::AllQuery, &TopDocs::with_limit(limit * 10)) {
                Ok(d) => d,
                Err(_) => return Vec::new(),
            };

        for (score, doc_address) in all_docs {
            let doc: tantivy::schema::TantivyDocument = searcher.doc(doc_address).unwrap();

            let title: String = doc
                .get_first(title_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let content: String = doc
                .get_first(content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let file_path: String = doc
                .get_first(file_path_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let relative_path: String = doc
                .get_first(relative_path_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let index_name: String = doc
                .get_first(index_name_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let language: String = doc
                .get_first(language_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let line_number: u32 = doc
                .get_first(line_number_field)
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;

            let symbols: Vec<SymbolInfo> = doc
                .get_first(symbols_field)
                .and_then(|v| v.as_str())
                .map(|s| serde_json::from_str(s).unwrap_or_default())
                .unwrap_or_default();

            let matching: Vec<_> = symbols
                .into_iter()
                .filter(|s| s.name.to_lowercase().contains(&symbol_name.to_lowercase()))
                .collect();

            if !matching.is_empty() {
                results.push(SearchResult {
                    title,
                    content,
                    file_path,
                    relative_path,
                    index_name,
                    language,
                    line_number,
                    score,
                    symbols: matching,
                });

                if results.len() >= limit {
                    break;
                }
            }
        }

        results
    }
}
