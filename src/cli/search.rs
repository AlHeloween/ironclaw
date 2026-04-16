//! Search service management commands.
//!
//! Provides subcommands for managing Local Code Search and Firecrawl Search services.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum SearchCommand {
    /// Start all configured search services
    #[command(about = "Start search services", long_about = "Starts Local Code Search and Firecrawl Search services if configured.\nExample: ironclaw search start")]
    Start,

    /// Stop all search services
    #[command(about = "Stop search services", long_about = "Stops all running search services.\nExample: ironclaw search stop")]
    Stop,

    /// Check service health
    #[command(about = "Check service health", long_about = "Checks if search services are running and healthy.\nExample: ironclaw search status")]
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List configured indexes
    #[command(about = "List search indexes", long_about = "Lists configured code indexes for local search.\nExample: ironclaw search indexes")]
    Indexes {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Index a codebase
    #[command(about = "Index a codebase", long_about = "Triggers indexing of a codebase directory.\nExample: ironclaw search index /path/to/code")]
    Index {
        /// Directory to index
        path: String,

        /// Index name (defaults to directory basename)
        #[arg(long)]
        name: Option<String>,

        /// Languages to index (default: all supported)
        #[arg(long, value_delimiter = ',')]
        languages: Vec<String>,
    },
}

pub async fn run_search_command(cmd: &SearchCommand) -> anyhow::Result<()> {
    use crate::cli::fmt;
    use std::path::PathBuf;

    match cmd {
        SearchCommand::Start => {
            let manager = crate::search_manager::SearchServiceManager::new();
            match manager.start_all().await {
                Ok(()) => {
                    println!("{}Search services started{}", fmt::success(), fmt::reset());
                }
                Err(e) => {
                    println!("{}Failed to start search services: {}{}", fmt::error(), e, fmt::reset());
                }
            }
        }

        SearchCommand::Stop => {
            let manager = crate::search_manager::SearchServiceManager::new();
            manager.stop_all().await;
            println!("{}Search services stopped{}", fmt::success(), fmt::reset());
        }

        SearchCommand::Status { json } => {
            let local_ok = check_service_health(3004).await;
            let firecrawl_ok = check_service_health(3005).await;

            if *json {
                let status = serde_json::json!({
                    "local_code_search": if local_ok { "healthy" } else { "unhealthy" },
                    "firecrawl_search": if firecrawl_ok { "healthy" } else { "unhealthy" },
                });
                println!("{}", serde_json::to_string_pretty(&status).unwrap());
            } else {
                let local_status = if local_ok {
                    format!("{}healthy{}", fmt::success(), fmt::reset())
                } else {
                    format!("{}unhealthy{}", fmt::error(), fmt::reset())
                };
                let fc_status = if firecrawl_ok {
                    format!("{}healthy{}", fmt::success(), fmt::reset())
                } else {
                    format!("{}unhealthy{}", fmt::error(), fmt::reset())
                };
                println!("Local Code Search (port 3004): {}", local_status);
                println!("Firecrawl Search (port 3005):  {}", fc_status);
            }
        }

        SearchCommand::Indexes { json } => {
            let config_path = ironclaw_base_dir().join("local-code-search.jsonc");
            if !config_path.exists() {
                if *json {
                    println!("{}", serde_json::json!({"error": "Local Code Search not configured"}));
                } else {
                    println!("{}Local Code Search not configured{}", fmt::warning(), fmt::reset());
                    println!("Create ~/.ironclaw/local-code-search.jsonc to configure indexes.");
                }
                return Ok(());
            }

            let content = std::fs::read_to_string(&config_path)?;
            let config: serde_json::Value = serde_json::from_str(&content)?;
            let indexes = config.get("indexes").and_then(|v| v.as_array());

            if let Some(indexes) = indexes {
                if *json {
                    println!("{}", serde_json::to_string_pretty(indexes).unwrap());
                } else {
                    println!("Configured indexes:");
                    for idx in indexes {
                        let name = idx.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                        let path = idx.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                        let enabled = idx.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                        let status = if enabled {
                            format!("{}enabled{}", fmt::success(), fmt::reset())
                        } else {
                            format!("{}disabled{}", fmt::dim(), fmt::reset())
                        };
                        println!("  {}  {} ({})", status, name, path);
                    }
                }
            } else if *json {
                println!("[]");
            } else {
                println!("No indexes configured.");
            }
        }

        SearchCommand::Index { path, name, languages } => {
            let local_ok = check_service_health(3004).await;
            if !local_ok {
                println!("{}Local Code Search service is not running{}", fmt::error(), fmt::reset());
                println!("Run 'ironclaw search start' first.");
                return Ok(());
            }

            let index_name = name.clone().unwrap_or_else(|| {
                PathBuf::from(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            });

            let url = format!(
                "http://127.0.0.1:3004/index/{}/index",
                urlencoding::encode(&index_name)
            );

            let payload = serde_json::json!({
                "path": path,
                "languages": if languages.is_empty() { vec!["all"] } else { languages.clone() },
                "symbols_enabled": true,
            });

            match reqwest::Client::new()
                .post(&url)
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    println!("{}Indexing started for '{}': {}{}", fmt::success(), index_name, path, fmt::reset());
                }
                Ok(resp) => {
                    println!("{}Indexing failed: HTTP {}{}", fmt::error(), resp.status(), fmt::reset());
                }
                Err(e) => {
                    println!("{}Indexing request failed: {}{}", fmt::error(), e, fmt::reset());
                }
            }
        }
    }

    Ok(())
}

async fn check_service_health(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{}/health", port);
    reqwest::get(&url)
        .await
        .ok()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn ironclaw_base_dir() -> PathBuf {
    dirs::home_dir()
        .map(|mut p| {
            p.push(".ironclaw");
            p
        })
        .expect("Could not determine home directory")
}
