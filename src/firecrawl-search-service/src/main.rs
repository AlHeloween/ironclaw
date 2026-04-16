//! Firecrawl Search Service - Native Rust binary.
//!
//! Provides web search, code search (Sourcegraph), URL scraping, and hybrid mode
//! combining local code search with public codebase search.
//!
//! # Usage
//!
//! ```bash
//! # Start with default config (~/.ironclaw/firecrawl-search.jsonc)
//! firecrawl-search-service
//!
//! # Start with custom config
//! firecrawl-search-service --config /path/to/config.jsonc
//!
//! # Override port
//! firecrawl-search-service --port 3005
//! ```

use clap::Parser;
use firecrawl_search_service::{FirecrawlSearchConfig, FirecrawlSearchService};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "firecrawl-search-service")]
#[command(about = "Firecrawl Search Service for IronClaw")]
struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Override service port
    #[arg(short, long)]
    port: Option<u16>,

    /// Override bind address
    #[arg(short, long)]
    bind: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let config = if let Some(config_path) = &args.config {
        FirecrawlSearchConfig::load(config_path)?
    } else {
        FirecrawlSearchConfig::load_default()?
    };

    let config = apply_overrides(config, &args);

    tracing::info!("Starting Firecrawl Search Service");
    tracing::info!("Firecrawl API: {}", config.firecrawl.api_url);
    tracing::info!("Sourcegraph API: {}", config.sourcegraph.api_url);
    tracing::info!(
        "Local search: {} (enabled: {})",
        config.local_search.url,
        config.local_search.enabled
    );

    let service = FirecrawlSearchService::new(config);
    service.run().await?;

    Ok(())
}

fn apply_overrides(mut config: FirecrawlSearchConfig, args: &Args) -> FirecrawlSearchConfig {
    if let Some(port) = args.port {
        config.service.port = port;
    }
    if let Some(bind) = &args.bind {
        config.service.bind_address = bind.clone();
    }
    config
}
