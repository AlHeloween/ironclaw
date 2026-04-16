//! Local Code Search Service for IronClaw.
//!
//! Provides full-text search and symbol search across local codebases.
//! Uses tantivy for indexing and regex-based symbol extraction.

pub mod config;
pub mod index;
pub mod search;
pub mod service;
pub mod symbols;

pub use config::LocalCodeSearchConfig;
pub use index::CodeIndex;
pub use search::{SearchQuery, SearchResult, SearchService};
pub use service::CodeSearchService;
