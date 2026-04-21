//! Web search module: Firecrawl API client, context scraping, Sourcegraph integration.

mod firecrawl;
mod context;
mod sourcegraph;

pub use firecrawl::{execute_web_search, WebSearchRequest};
pub use context::{execute_context, ContextRequest};
pub use sourcegraph::{execute_sourcegraph, SourcegraphRequest};
