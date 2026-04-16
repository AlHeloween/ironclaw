//! Native Firecrawl Search Service for IronClaw.
//!
//! Provides web search, code search (Sourcegraph), URL scraping, and hybrid mode
//! combining local code search with public codebase search.
//!
//! # Configuration
//!
//! Config file: `~/.ironclaw/firecrawl-search.jsonc`
//!
//! ```jsonc
//! {
//!   "service": {
//!     "port": 3005,
//!     "bind_address": "127.0.0.1"
//!   },
//!   "firecrawl": {
//!     "api_url": "http://localhost:3002"
//!   },
//!   "sourcegraph": {
//!     "access_token": ""
//!   },
//!   "local_search": {
//!     "url": "http://127.0.0.1:3004"
//!   }
//! }
//! ```

pub mod config;
pub mod context;
pub mod hybrid;
pub mod search;
pub mod service;
pub mod sourcegraph;

pub use config::FirecrawlSearchConfig;
pub use service::FirecrawlSearchService;
