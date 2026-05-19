//! Bootstrap: auto-clone Firecrawl, check dependencies.

use crate::config::Config;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct BootstrapResult {
    pub firecrawl_cloned: bool,
    pub firecrawl_installed: bool,
    pub firecrawl_started: bool,
    pub postgres_available: bool,
    pub redis_available: bool,
    pub firecrawl_db_ready: bool,
    pub warnings: Vec<String>,
}

pub fn run_bootstrap(config: &Config) -> BootstrapResult {
    let mut result = BootstrapResult {
        firecrawl_cloned: false,
        firecrawl_installed: false,
        firecrawl_started: false,
        postgres_available: false,
        redis_available: false,
        firecrawl_db_ready: false,
        warnings: Vec::new(),
    };

    // Check PostgreSQL
    result.postgres_available = check_postgres();
    if !result.postgres_available {
        result.warnings.push(
            "PostgreSQL not available on localhost:5432 — web search may not work".to_string(),
        );
    }

    // Ensure Firecrawl database exists
    if result.postgres_available {
        if let Some(ref pg_config) = config.web_search.firecrawl.postgres {
            result.firecrawl_db_ready = ensure_firecrawl_db(pg_config);
            if !result.firecrawl_db_ready {
                result.warnings.push(
                    "Firecrawl database could not be created — web search will fail".to_string(),
                );
            }
        } else {
            // No postgres config — can't create DB, but PG is available
            result.firecrawl_db_ready = false;
        }
    }

    // Check Redis
    result.redis_available = check_redis();
    if !result.redis_available {
        result
            .warnings
            .push("Redis not available on localhost:6379 — web search may not work".to_string());
    }

    if !result.postgres_available || !result.redis_available {
        tracing::warn!("PostgreSQL and/or Redis not available — Firecrawl requires both");
        return result;
    }

    // Clone Firecrawl if needed
    if config.bootstrap.auto_clone_firecrawl {
        let repo_path = match config.firecrawl_repo_path() {
            Ok(p) => p,
            Err(e) => {
                result
                    .warnings
                    .push(format!("Could not resolve Firecrawl repo path: {}", e));
                return result;
            }
        };

        if !repo_path.exists() {
            tracing::info!("Cloning Firecrawl repo to {}", repo_path.display());
            let commit = config
                .web_search
                .firecrawl
                .commit
                .as_deref()
                .unwrap_or("main");
            match clone_firecrawl(&repo_path, commit) {
                Ok(_) => {
                    fix_firecrawl_workspace(&repo_path);
                    patch_firecrawl_harness(&repo_path.join("apps").join("api"));
                    result.firecrawl_cloned = true;
                }
                Err(e) => result
                    .warnings
                    .push(format!("Failed to clone Firecrawl: {}", e)),
            }
        } else {
            result.firecrawl_cloned = true;
        }
    }

    // Install Firecrawl deps if repo exists
    if result.firecrawl_cloned && config.bootstrap.auto_install_deps {
        let repo_path = match config.firecrawl_repo_path() {
            Ok(p) => p,
            Err(_) => return result,
        };
        let api_dir = repo_path.join("apps").join("api");
        if api_dir.exists() {
            tracing::info!("Installing Firecrawl dependencies...");
            match install_firecrawl_deps(&api_dir) {
                Ok(_) => result.firecrawl_installed = true,
                Err(e) => result
                    .warnings
                    .push(format!("Failed to install Firecrawl deps: {}", e)),
            }
        }
    }

    // Start Firecrawl if configured and deps are ready
    if config.web_search.firecrawl.auto_start && result.firecrawl_installed {
        let repo_path = match config.firecrawl_repo_path() {
            Ok(p) => p,
            Err(_) => return result,
        };
        let api_dir = repo_path.join("apps").join("api");
        if api_dir.exists() {
            tracing::info!("Starting Firecrawl server...");
            match start_firecrawl_server(&api_dir) {
                Ok(_) => result.firecrawl_started = true,
                Err(e) => result
                    .warnings
                    .push(format!("Failed to start Firecrawl: {}", e)),
            }
        }
    }

    result
}

fn check_postgres() -> bool {
    use std::net::TcpStream;
    TcpStream::connect("127.0.0.1:5432").is_ok()
}

fn check_redis() -> bool {
    use std::net::TcpStream;
    TcpStream::connect("127.0.0.1:6379").is_ok()
}

/// Ensure the Firecrawl database exists in PostgreSQL.
/// Returns true if it already exists or was successfully created.
fn ensure_firecrawl_db(pg: &crate::config::PostgresConfig) -> bool {
    let psql = match find_psql() {
        Some(p) => p,
        None => {
            tracing::warn!("psql not found in PATH — cannot check/create Firecrawl database");
            return false;
        }
    };

    let db_name = &pg.database;

    // Step 1: check if the database already exists
    match run_psql_cmd(&psql, pg, db_name, "SELECT 1;") {
        Ok(true) => {
            tracing::info!("Firecrawl database '{}' already exists", db_name);
            return true;
        }
        Ok(false) => {
            // psql ran but returned an error — could be "database does not exist"
            // or could be another issue. Try creating the DB regardless.
        }
        Err(_) => {
            // psql failed to launch — will try creation anyway below
        }
    }

    // Step 2: create the database
    tracing::info!(
        "Firecrawl database '{}' not found, creating on {}:{}",
        db_name,
        pg.host,
        pg.port
    );

    let create_sql = format!("CREATE DATABASE \"{}\";", db_name);
    match run_psql_cmd(&psql, pg, "postgres", &create_sql) {
        Ok(true) => {
            tracing::info!("Firecrawl database '{}' created successfully", db_name);
            true
        }
        Ok(false) => {
            tracing::error!(
                "Failed to create Firecrawl database '{}' — psql returned an error",
                db_name
            );
            false
        }
        Err(e) => {
            tracing::error!("Failed to create Firecrawl database '{}': {}", db_name, e);
            false
        }
    }
}

/// Run a psql command and return Ok(true) on success, Ok(false) on SQL error.
fn run_psql_cmd(psql: &Path, pg: &crate::config::PostgresConfig, db: &str, sql: &str) -> anyhow::Result<bool> {
    let mut cmd = Command::new(psql);
    cmd.arg("-h")
        .arg(&pg.host)
        .arg("-p")
        .arg(pg.port.to_string())
        .arg("-U")
        .arg(&pg.username)
        .arg("-d")
        .arg(db)
        .arg("-w")
        .arg("-c")
        .arg(sql);

    if let Some(ref pwd) = pg.password {
        cmd.env("PGPASSWORD", pwd);
    }

    let output = cmd.stdin(Stdio::null()).output()?;
    if output.status.success() {
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If psql says the database doesn't exist, that's an expected error
        if stderr.contains("does not exist") {
            tracing::debug!("Database '{}' check: {}", db, stderr.trim());
        } else {
            tracing::warn!("psql error (db='{}'): {}", db, stderr.trim());
        }
        Ok(false)
    }
}

/// Find psql executable in common locations or via PATH.
fn find_psql() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    // Check common install paths on Windows
    #[cfg(windows)]
    {
        let candidates = &[
            r"C:\Program Files\PostgreSQL\17\bin\psql.exe",
            r"C:\Program Files\PostgreSQL\16\bin\psql.exe",
            r"C:\Program Files\PostgreSQL\15\bin\psql.exe",
            r"C:\Program Files\PostgreSQL\14\bin\psql.exe",
            r"d:\USESoft\PostgreSQL\bin\psql.exe",
        ];
        for c in candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Some(p);
            }
        }
    }

    #[cfg(unix)]
    {
        let candidates = &[
            "/usr/bin/psql",
            "/usr/local/bin/psql",
            "/opt/homebrew/bin/psql",
        ];
        for c in candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Some(p);
            }
        }
    }

    // Fall back: try bare "psql" command (relies on PATH)
    if Command::new("psql").arg("--version").output().is_ok() {
        return Some(PathBuf::from("psql"));
    }

    None
}

fn clone_firecrawl(repo_path: &Path, commit: &str) -> anyhow::Result<()> {
    if let Some(parent) = repo_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let depth = if commit == "main" { "--depth=1" } else { "" };
    let output = Command::new("git")
        .args(["clone", depth, "https://github.com/firecrawl/firecrawl.git"])
        .arg(repo_path)
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git clone failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    if commit != "main" {
        let output = Command::new("git")
            .args(["checkout", commit])
            .current_dir(repo_path)
            .stdin(Stdio::null())
            .output()?;
        if !output.status.success() {
            anyhow::bail!(
                "git checkout failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    Ok(())
}

fn install_firecrawl_deps(api_dir: &Path) -> anyhow::Result<()> {
    let output = Command::new("pnpm")
        .arg("install")
        .current_dir(api_dir)
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "pnpm install failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn fix_firecrawl_workspace(repo_path: &Path) {
    let native_cargo = repo_path
        .join("apps")
        .join("api")
        .join("native")
        .join("Cargo.toml");
    if !native_cargo.exists() {
        return;
    }
    let content = match std::fs::read_to_string(&native_cargo) {
        Ok(c) => c,
        Err(_) => return,
    };
    if content.contains("[workspace]") {
        return;
    }
    let new_content = content.replacen("[package]", "[package]\n\n[workspace]\n", 1);
    let _ = std::fs::write(&native_cargo, new_content);
    tracing::info!("Fixed Firecrawl native Cargo.toml workspace isolation");
}

fn patch_firecrawl_harness(api_dir: &Path) {
    let harness_ts = api_dir.join("src").join("harness.ts");
    if !harness_ts.exists() {
        return;
    }
    let content = match std::fs::read_to_string(&harness_ts) {
        Ok(c) => c,
        Err(_) => return,
    };

    if !content.contains("Installing Go dependencies") {
        return;
    }

    let mut new_content = content.clone();

    let install_line = "    logger.info(\"Installing Go dependencies\");";
    if new_content.contains(install_line) {
        let wrapper = format!(
            "    if (!process.env.USE_GO_MARKDOWN_PARSER || process.env.USE_GO_MARKDOWN_PARSER === 'true') {{\n    "
        );
        new_content = new_content.replacen(install_line, &wrapper, 1);
    }

    let build_line = "    logger.info(\"Building Go module\");";
    if new_content.contains(build_line) {
        new_content = new_content.replacen(build_line, build_line, 1);
    }

    let exec_forward_after_build = "    const build = execForward(\"go@build\",";
    if let Some(pos) = new_content.find(exec_forward_after_build) {
        let newline = &new_content[pos..];
        if let Some(end_brace) = newline.find('\n') {
            let insert_pos = pos + end_brace + 1;
            let closing = "    }";
            new_content.insert_str(insert_pos, closing);
        }
    }

    if new_content != content {
        let _ = std::fs::write(&harness_ts, new_content);
        tracing::info!(
            "Patched Firecrawl harness.ts to skip Go build when USE_GO_MARKDOWN_PARSER=false"
        );
    }
}

fn start_firecrawl_server(api_dir: &Path) -> anyhow::Result<()> {
    let mut env = std::env::vars().collect::<std::collections::HashMap<_, _>>();
    env.insert("USE_GO_MARKDOWN_PARSER".to_string(), "false".to_string());

    Command::new("pnpm")
        .arg("run")
        .arg("start")
        .current_dir(api_dir)
        .envs(env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    tracing::info!("Firecrawl server started (detached, USE_GO_MARKDOWN_PARSER=false)");
    Ok(())
}
