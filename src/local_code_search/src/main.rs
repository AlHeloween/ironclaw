//! Local Code Search Service Binary

use clap::Parser;
use local_code_search::{CodeSearchService, LocalCodeSearchConfig};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "local-code-search")]
#[command(about = "Local code search service for IronClaw")]
struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Port to listen on (overrides config)
    #[arg(short, long)]
    port: Option<u16>,

    /// Bind address (overrides config)
    #[arg(short, long)]
    bind: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "local_code_search=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config_path = args.config.unwrap_or_else(|| {
        dirs::home_dir()
            .map(|mut p| {
                p.push(".ironclaw/local-code-search.jsonc");
                p
            })
            .expect("Could not determine home directory")
    });

    tracing::info!("Loading configuration from {:?}", config_path);
    let config = LocalCodeSearchConfig::load(&config_path)?;

    let config = if let Some(port) = args.port {
        config.with_port(port)
    } else {
        config
    };
    let config = if let Some(bind) = args.bind {
        config.with_bind_address(bind)
    } else {
        config
    };

    tracing::info!(
        "Starting Local Code Search Service on {}:{}",
        config.service.port, config.service.bind_address
    );

    let service = CodeSearchService::new(config);
    service.run().await?;

    Ok(())
}
