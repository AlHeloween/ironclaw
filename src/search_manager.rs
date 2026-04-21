//! Search Service Manager — auto-starts and manages the universal-search-service.
//!
//! When IronClaw starts, this manager:
//! 1. Checks if config file exists (~/.ironclaw/universal-search.jsonc)
//! 2. Spawns the universal-search-service binary as a child process
//! 3. Monitors its health via HTTP health checks on ports 3004 (local) and 3005 (web)
//! 4. Kills it on shutdown

use std::path::PathBuf;
use std::process::Child;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

pub struct SearchServiceManager {
    universal_search_child: Arc<Mutex<Option<Child>>>,
    health_check_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl SearchServiceManager {
    pub fn new() -> Self {
        Self {
            universal_search_child: Arc::new(Mutex::new(None)),
            health_check_handles: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Start all configured search services.
    pub async fn start_all(&self) -> anyhow::Result<()> {
        let config_path = ironclaw_base_dir().join("universal-search.jsonc");
        if !config_path.exists() {
            info!("Universal Search Service not configured (no ~/.ironclaw/universal-search.jsonc)");
            return Ok(());
        }

        let binary = find_binary("universal-search-service")?;

        info!(
            "Starting Universal Search Service: {} --config {:?}",
            binary.display(),
            config_path
        );

        let child = std::process::Command::new(&binary)
            .arg("--config")
            .arg(&config_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        let mut child_guard = self.universal_search_child.lock().await;
        *child_guard = Some(child);
        drop(child_guard);

        tokio::time::sleep(STARTUP_TIMEOUT).await;

        self.start_health_check("universal-search (local)", 3004).await;
        self.start_health_check("universal-search (web)", 3005).await;

        info!("Universal Search Service started");
        Ok(())
    }

    /// Start periodic health check for a service port.
    async fn start_health_check(&self, name: &'static str, port: u16) {
        let health_url = format!("http://127.0.0.1:{}/health", port);
        let handles = self.health_check_handles.clone();

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(HEALTH_CHECK_INTERVAL).await;
                match reqwest::get(&health_url).await {
                    Ok(resp) if resp.status().is_success() => {
                        tracing::debug!("{} health check: OK", name);
                    }
                    Ok(resp) => {
                        warn!("{} health check: HTTP {}", name, resp.status());
                    }
                    Err(e) => {
                        warn!("{} health check failed: {}", name, e);
                    }
                }
            }
        });

        let mut handles_guard = handles.lock().await;
        handles_guard.push(handle);
    }

    /// Stop all search services.
    pub async fn stop_all(&self) {
        let handles = std::mem::take(&mut *self.health_check_handles.lock().await);
        for handle in handles {
            handle.abort();
        }

        if let Some(mut child) = self.universal_search_child.lock().await.take() {
            info!("Stopping Universal Search Service...");
            let _ = child.kill();
            let _ = child.wait();
        }

        info!("All search services stopped");
    }

    /// Check if the universal search service is running.
    pub async fn is_running(&self) -> bool {
        self.universal_search_child.lock().await.is_some()
    }
}

/// Find a binary in PATH or in a subdirectory next to the ironclaw binary.
fn find_binary(name: &str) -> anyhow::Result<PathBuf> {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let binary = parent
                .join(name)
                .join(name)
                .with_extension(std::env::consts::EXE_EXTENSION);
            if binary.exists() {
                return Ok(binary);
            }
            let flat = parent
                .join(name)
                .with_extension(std::env::consts::EXE_EXTENSION);
            if flat.exists() {
                return Ok(flat);
            }
        }
    }

    if let Ok(path) = which::which(name) {
        return Ok(path);
    }

    anyhow::bail!(
        "Could not find '{}' binary. It should be in a subdirectory next to ironclaw or in PATH.",
        name
    )
}

fn ironclaw_base_dir() -> PathBuf {
    dirs::home_dir()
        .map(|mut p| {
            p.push(".ironclaw");
            p
        })
        .expect("Could not determine home directory")
}
