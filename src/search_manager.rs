//! Search Service Manager — auto-starts and manages local-code-search and firecrawl-search-service.
//!
//! When IronClaw starts, this manager:
//! 1. Checks if config files exist (~/.ironclaw/local-code-search.jsonc and ~/.ironclaw/firecrawl-search.jsonc)
//! 2. Spawns the corresponding service binaries as child processes
//! 3. Monitors their health via HTTP health checks
//! 4. Kills them on shutdown

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
    local_code_search_child: Arc<Mutex<Option<Child>>>,
    firecrawl_search_child: Arc<Mutex<Option<Child>>>,
    health_check_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl SearchServiceManager {
    pub fn new() -> Self {
        Self {
            local_code_search_child: Arc::new(Mutex::new(None)),
            firecrawl_search_child: Arc::new(Mutex::new(None)),
            health_check_handles: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Start all configured search services.
    pub async fn start_all(&self) -> anyhow::Result<()> {
        let base_dir = ironclaw_base_dir();

        // Start Local Code Search if configured
        let lcs_config = base_dir.join("local-code-search.jsonc");
        if lcs_config.exists() {
            self.start_local_code_search().await?;
        } else {
            info!("Local Code Search not configured (no ~/.ironclaw/local-code-search.jsonc)");
        }

        // Start Firecrawl Search if configured
        let fcs_config = base_dir.join("firecrawl-search.jsonc");
        if fcs_config.exists() {
            self.start_firecrawl_search().await?;
        } else {
            info!("Firecrawl Search not configured (no ~/.ironclaw/firecrawl-search.jsonc)");
        }

        Ok(())
    }

    /// Start Local Code Search service.
    async fn start_local_code_search(&self) -> anyhow::Result<()> {
        let binary = find_binary("local-code-search")?;
        let config_path = ironclaw_base_dir().join("local-code-search.jsonc");

        info!(
            "Starting Local Code Search Service: {} --config {:?}",
            binary.display(),
            config_path
        );

        let child = std::process::Command::new(&binary)
            .arg("--config")
            .arg(&config_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        let mut child_guard = self.local_code_search_child.lock().await;
        *child_guard = Some(child);
        drop(child_guard);

        // Wait for service to start
        tokio::time::sleep(STARTUP_TIMEOUT).await;

        // Start health check
        self.start_health_check("local-code-search", 3004).await;

        info!("Local Code Search Service started");
        Ok(())
    }

    /// Start Firecrawl Search service.
    async fn start_firecrawl_search(&self) -> anyhow::Result<()> {
        let binary = find_binary("firecrawl-search-service")?;
        let config_path = ironclaw_base_dir().join("firecrawl-search.jsonc");

        info!(
            "Starting Firecrawl Search Service: {} --config {:?}",
            binary.display(),
            config_path
        );

        let child = std::process::Command::new(&binary)
            .arg("--config")
            .arg(&config_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        let mut child_guard = self.firecrawl_search_child.lock().await;
        *child_guard = Some(child);
        drop(child_guard);

        // Wait for service to start
        tokio::time::sleep(STARTUP_TIMEOUT).await;

        // Start health check
        self.start_health_check("firecrawl-search-service", 3005)
            .await;

        info!("Firecrawl Search Service started");
        Ok(())
    }

    /// Start periodic health check for a service.
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
        // Stop health checks
        let handles = std::mem::take(&mut *self.health_check_handles.lock().await);
        for handle in handles {
            handle.abort();
        }

        // Kill Local Code Search
        if let Some(mut child) = self.local_code_search_child.lock().await.take() {
            info!("Stopping Local Code Search Service...");
            let _ = child.kill();
            let _ = child.wait();
        }

        // Kill Firecrawl Search
        if let Some(mut child) = self.firecrawl_search_child.lock().await.take() {
            info!("Stopping Firecrawl Search Service...");
            let _ = child.kill();
            let _ = child.wait();
        }

        info!("All search services stopped");
    }

    /// Check if Local Code Search is running.
    pub async fn is_local_code_search_running(&self) -> bool {
        self.local_code_search_child.lock().await.is_some()
    }

    /// Check if Firecrawl Search is running.
    pub async fn is_firecrawl_search_running(&self) -> bool {
        self.firecrawl_search_child.lock().await.is_some()
    }
}

/// Find a binary in PATH or in a subdirectory next to the ironclaw binary.
fn find_binary(name: &str) -> anyhow::Result<PathBuf> {
    // First, try to find in a subdirectory next to the ironclaw binary
    // (e.g. ironclaw.exe is at /opt/ironclaw/ironclaw, service at /opt/ironclaw/local-code-search/local-code-search)
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let binary = parent
                .join(name)
                .join(name)
                .with_extension(std::env::consts::EXE_EXTENSION);
            if binary.exists() {
                return Ok(binary);
            }
            // Also try flat: same directory as ironclaw
            let flat = parent
                .join(name)
                .with_extension(std::env::consts::EXE_EXTENSION);
            if flat.exists() {
                return Ok(flat);
            }
        }
    }

    // Then try PATH
    if let Ok(path) = which::which(name) {
        return Ok(path);
    }

    anyhow::bail!(
        "Could not find '{}' binary. It should be in a subdirectory next to ironclaw or in PATH.",
        name
    )
}

/// Get the IronClaw base directory (~/.ironclaw).
fn ironclaw_base_dir() -> PathBuf {
    dirs::home_dir()
        .map(|mut p| {
            p.push(".ironclaw");
            p
        })
        .expect("Could not determine home directory")
}
