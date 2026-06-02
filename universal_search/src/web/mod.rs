//! Web search module: Firecrawl API client, context scraping, Sourcegraph integration, direct HTTPS fetch, Playwright browser.

mod firecrawl;
mod context;
mod sourcegraph;
mod fetch;
mod browser_fetch;

pub use firecrawl::{execute_web_search, WebSearchRequest};
pub use context::{execute_context, ContextRequest};
pub use sourcegraph::{execute_sourcegraph, SourcegraphRequest};
pub use fetch::{execute_fetch, FetchRequest};
pub use browser_fetch::{execute_browser_fetch, BrowserRequest};
