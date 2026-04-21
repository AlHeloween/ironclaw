//! Universal Search Service - main entry point.

mod bootstrap;
mod config;
mod hybrid;
mod ring_log;
mod service;
mod web;

use clap::{Parser, Subcommand};
use ring_log::SharedRingLog;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::Level;

#[derive(Parser)]
#[command(name = "universal-search-service")]
#[command(about = "Universal search service: web search and code search aggregation")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Log level: trace, debug, info, warn, error
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Log file path (relative to exe dir or absolute). Uses 10KB circular buffer.
    #[arg(long)]
    log_file: Option<PathBuf>,

    /// Wait for debugger on startup (Windows: MessageBox, Unix: 30s delay)
    #[arg(long, hide = true)]
    wait_debugger: bool,

    /// Path to configuration file
    #[arg(long, short = 'c')]
    config: Option<PathBuf>,

    /// Override service port
    #[arg(long, short = 'p')]
    port: Option<u16>,

    /// Override bind address
    #[arg(long, short = 'b')]
    bind: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in foreground with logging (default if no command)
    Run,
    /// Start as daemon (background)
    Start,
    /// Stop the running service
    Stop,
    /// Check service status
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Run diagnostics
    Diag,
    /// Show or create configuration
    Config {
        #[arg(long)]
        create: bool,
    },
    /// Bootstrap dependencies (clone Firecrawl, check PostgreSQL/Redis)
    Bootstrap,
    /// Install Firecrawl
    InstallFirecrawl,
    /// Register as OS service
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },
    /// Convert config paths between relative and absolute
    ConvertPaths {
        #[arg(long)]
        to: String,
    },
    /// View service logs (circular buffer)
    Logs {
        /// Number of lines to show
        #[arg(long, short = 'n', default_value = "50")]
        tail: usize,
        /// Follow log output (like tail -f)
        #[arg(long, short = 'f')]
        follow: bool,
    },
}

#[derive(clap::Subcommand)]
enum ServiceAction {
    /// Register as OS service
    Install,
    /// Start the OS service
    Start,
    /// Stop the OS service
    Stop,
    /// Check OS service status
    Status,
    /// Uninstall OS service
    Uninstall,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.wait_debugger {
        wait_for_debugger();
    }

    let _guard = init_logging(&cli.log_level, cli.log_file.as_ref());

    match cli.command {
        None => run_foreground(cli),
        Some(Commands::Run) => run_foreground(cli),
        Some(Commands::Start) => run_daemon(&cli),
        Some(Commands::Stop) => stop_service(),
        Some(Commands::Status { json }) => show_status(json),
        Some(Commands::Diag) => run_diagnostics(),
        Some(Commands::Config { create }) => show_config(create),
        Some(Commands::Bootstrap) => run_bootstrap_cmd(),
        Some(Commands::InstallFirecrawl) => install_firecrawl(),
        Some(Commands::Service { action }) => handle_service_action(action),
        Some(Commands::ConvertPaths { to }) => convert_paths(&to),
        Some(Commands::Logs { tail, follow }) => show_logs(tail, follow),
    }
}

fn parse_level(s: &str) -> Level {
    match s.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

struct LoggingGuard {
    _file_guard: Option<SharedRingLog>,
}

fn init_logging(log_level: &str, log_file: Option<&PathBuf>) -> LoggingGuard {
    let level = parse_level(log_level);

    if let Some(path) = log_file {
        let resolved = match config::exe_dir() {
            Ok(base) => {
                let path_str = path.to_string_lossy();
                config::resolve_path(&base, &path_str)
            }
            Err(_) => path.clone(),
        };

        if let Some(parent) = resolved.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let ring_writer = match ring_log::RingLogWriter::open(&resolved) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to open log file {}: {}", resolved.display(), e);
                tracing_subscriber::fmt::init();
                return LoggingGuard { _file_guard: None };
            }
        };

        let shared = Arc::new(Mutex::new(ring_writer));
        let writer = shared.clone();

        tracing_subscriber::fmt()
            .with_max_level(level)
            .with_writer(move || {
                struct MutexWriter(Arc<Mutex<ring_log::RingLogWriter>>);
                impl std::io::Write for MutexWriter {
                    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                        self.0.lock().unwrap().write(buf)
                    }
                    fn flush(&mut self) -> std::io::Result<()> {
                        self.0.lock().unwrap().flush()
                    }
                }
                MutexWriter(writer.clone())
            })
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_line_number(true)
            .init();

        init_panic_hook(shared.clone());

        LoggingGuard {
            _file_guard: Some(shared),
        }
    } else {
        tracing_subscriber::fmt()
            .with_max_level(level)
            .with_ansi(true)
            .init();
        LoggingGuard { _file_guard: None }
    }
}

fn init_panic_hook(log_writer: SharedRingLog) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("unnamed");

        let msg = match info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };

        let location = info
            .location()
            .map(|l| l.to_string())
            .unwrap_or_default();

        let panic_msg = format!(
            "PANIC [{}] {} - {}",
            thread_name, location, msg
        );

        if let Ok(mut writer) = log_writer.lock() {
            use std::io::Write;
            let _ = writer.write_log(panic_msg.as_bytes());
            let _ = writer.flush();
        }

        default_hook(info);
    }));
}

fn wait_for_debugger() {
    #[cfg(windows)]
    {
        use std::ffi::CString;
        use std::ptr::null_mut;
        unsafe {
            let text = CString::new("Attach debugger now!").unwrap();
            let caption = CString::new("universal-search-service").unwrap();
            winapi::um::winuser::MessageBoxA(
                null_mut(),
                text.as_ptr() as *const _,
                caption.as_ptr() as *const _,
                0,
            );

            while winapi::um::debugapi::IsDebuggerPresent() == 0 {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }
    #[cfg(unix)]
    {
        eprintln!(
            "Waiting 30s for debugger attachment... (PID: {})",
            std::process::id()
        );
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
    #[cfg(not(any(windows, unix)))]
    {
        eprintln!("--wait-debugger is only supported on Windows and Unix");
    }
}

fn show_logs(tail: usize, follow: bool) -> anyhow::Result<()> {
    let log_path = match config::exe_dir() {
        Ok(base) => base.join("logs").join("universal-search.log"),
        Err(_) => PathBuf::from("logs/universal-search.log"),
    };

    if !log_path.exists() {
        eprintln!("No log file found at {}", log_path.display());
        return Ok(());
    }

    if follow {
        use std::io::{BufRead, BufReader};
        use std::time::Duration;

        println!("Following log file: {}", log_path.display());
        loop {
            if let Ok(file) = std::fs::File::open(&log_path) {
                let reader = BufReader::new(file);
                for line in reader.lines().map_while(Result::ok) {
                    println!("{}", line);
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    } else {
        use std::io::{BufRead, BufReader};
        let file = std::fs::File::open(&log_path)?;
        let reader = BufReader::new(file);
        let lines: Vec<_> = reader.lines().map_while(Result::ok).collect();
        let start = lines.len().saturating_sub(tail);
        for line in &lines[start..] {
            println!("{}", line);
        }
    }

    Ok(())
}

fn run_foreground(cli: Cli) -> anyhow::Result<()> {
    let mut config = if let Some(ref path) = cli.config {
        config::Config::load(path).unwrap_or_default()
    } else {
        config::Config::load_default()?
    };

    if let Some(port) = cli.port { config.service.port = port; }
    if let Some(ref bind) = cli.bind { config.service.bind_address = bind.clone(); }

    if config.web_search.firecrawl.source == "local" {
        let bootstrap_result = bootstrap::run_bootstrap(&config);
        for warning in &bootstrap_result.warnings {
            tracing::warn!("Bootstrap: {}", warning);
            eprintln!("Warning: {}", warning);
        }
        if bootstrap_result.postgres_available {
            tracing::info!("PostgreSQL available on localhost:5432");
        }
        if bootstrap_result.redis_available {
            tracing::info!("Redis available on localhost:6379");
        }
        if bootstrap_result.firecrawl_cloned {
            tracing::info!("Firecrawl repo available at configured path");
        }
        if bootstrap_result.firecrawl_installed {
            tracing::info!("Firecrawl dependencies installed");
        }
        if bootstrap_result.firecrawl_started {
            tracing::info!("Firecrawl server started");
        }
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let http_service = service::SearchHttpService::new(config);
        http_service.run().await.map_err(anyhow::Error::from)
    })
}

fn run_daemon(_cli: &Cli) -> anyhow::Result<()> {
    use std::process::{Command, Stdio};
    let exe = std::env::current_exe()?;
    let child = Command::new(exe)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()?;
    println!("Service started as daemon (PID {})", child.id());
    Ok(())
}

fn stop_service() -> anyhow::Result<()> {
    println!("Stop not yet implemented - kill the process manually");
    Ok(())
}

fn show_status(json: bool) -> anyhow::Result<()> {
    let url = format!("http://127.0.0.1:{}/health", config::Config::load_default().map(|c| c.service.port).unwrap_or(3005));
    match reqwest::blocking::get(&url) {
        Ok(resp) if resp.status().is_success() => {
            if json {
                println!(r#"{{"status":"running","healthy":true}}"#);
            } else {
                println!("✓ Service is running and healthy");
            }
        }
        _ => {
            if json {
                println!(r#"{{"status":"not_running","healthy":false}}"#);
            } else {
                println!("✗ Service is not running");
            }
        }
    }
    Ok(())
}

fn run_diagnostics() -> anyhow::Result<()> {
    println!("Universal Search Service Diagnostics\n");
    let config = config::Config::load_default().unwrap_or_default();
    println!("Config: {}", config::Config::default_config_path()?.display());
    println!("{}", config.masked_display());

    let bootstrap_result = bootstrap::run_bootstrap(&config);
    println!("\nBootstrap:");
    println!("  Firecrawl cloned: {}", bootstrap_result.firecrawl_cloned);
    println!("  Firecrawl installed: {}", bootstrap_result.firecrawl_installed);
    println!("  PostgreSQL: {}", if bootstrap_result.postgres_available { "✓" } else { "✗" });
    println!("  Redis: {}", if bootstrap_result.redis_available { "✓" } else { "✗" });

    for warning in &bootstrap_result.warnings {
        println!("  ⚠ {}", warning);
    }
    Ok(())
}

fn show_config(create: bool) -> anyhow::Result<()> {
    let path = config::Config::default_config_path()?;
    if create {
        if path.exists() {
            println!("Config already exists at {}", path.display());
        } else {
            config::Config::write_default(&path)?;
            println!("Created default config at {}", path.display());
        }
    } else {
        if path.exists() {
            let config = config::Config::load(&path)?;
            println!("{}", config.masked_display());
        } else {
            println!("No config file found at {}", path.display());
            println!("Run with --config to create one.");
        }
    }
    Ok(())
}

fn run_bootstrap_cmd() -> anyhow::Result<()> {
    let config = config::Config::load_default()?;
    let result = bootstrap::run_bootstrap(&config);
    println!("Bootstrap complete:");
    println!("  Firecrawl cloned: {}", result.firecrawl_cloned);
    println!("  Firecrawl installed: {}", result.firecrawl_installed);
    for w in &result.warnings { println!("  ⚠ {}", w); }
    Ok(())
}

fn install_firecrawl() -> anyhow::Result<()> {
    let config = config::Config::load_default()?;
    let result = bootstrap::run_bootstrap(&config);
    if result.firecrawl_cloned && result.firecrawl_installed {
        println!("Firecrawl installed successfully");
    } else {
        for w in &result.warnings { println!("⚠ {}", w); }
    }
    Ok(())
}

#[allow(unused)]
fn handle_service_action(action: ServiceAction) -> anyhow::Result<()> {
    let service_name = "universal-search-service";
    let exe = std::env::current_exe()?;

    match action {
        ServiceAction::Install => {
            let log_dir = match config::exe_dir() {
                Ok(base) => base.join("logs"),
                Err(_) => PathBuf::from("logs"),
            };
            let _ = std::fs::create_dir_all(&log_dir);
            let log_path = log_dir.join("universal-search.log");

            #[cfg(windows)]
            {
                let cmd = format!(
                    "\"{}\" run --log-file \"{}\" --log-level info",
                    exe.display(),
                    log_path.display()
                );
                let output = std::process::Command::new("sc")
                    .args([
                        "create",
                        service_name,
                        "binPath=",
                        &cmd,
                        "start=",
                        "auto",
                        "obj=",
                        r".\CurrentUser",
                    ])
                    .output()?;
                if !output.status.success() {
                    eprintln!("Failed to create service: {}", String::from_utf8_lossy(&output.stderr));
                    eprintln!("Note: This command requires Administrator privileges.");
                    anyhow::bail!("sc create failed");
                }
                println!("Service '{}' registered", service_name);
            }
            #[cfg(target_os = "macos")]
            {
                let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
                let plist_dir = home.join("Library").join("LaunchAgents");
                std::fs::create_dir_all(&plist_dir)?;
                let plist_path = plist_dir.join("com.universalsearch.daemon.plist");
                let plist = format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
 dict<key>Label</key><string>com.universalsearch.daemon</string>
<key>ProgramArguments</key><array><string>{}</string><string>run</string><string>--log-file</string><string>{}</string><string>--log-level</string><string>info</string></array>
<key>RunAtLoad</key><true/><key>KeepAlive</key><true/>
</dict></plist>"#,
                    exe.display(),
                    log_path.display()
                );
                std::fs::write(&plist_path, plist)?;
                let _ = std::process::Command::new("launchctl")
                    .args(["load", "-w"])
                    .arg(&plist_path)
                    .output();
                println!("Service registered on macOS");
            }
            #[cfg(target_os = "linux")]
            {
                let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
                let unit_dir = home.join(".config").join("systemd").join("user");
                std::fs::create_dir_all(&unit_dir)?;
                let unit_path = unit_dir.join("universal-search.service");
                let unit = format!(
                    r#"[Unit]
Description=Universal Search Service
After=network.target

[Service]
Type=simple
ExecStart={} run --log-file {} --log-level info
Restart=always
RestartSec=3

[Install]
WantedBy=default.target"#,
                    exe.display(),
                    log_path.display()
                );
                std::fs::write(&unit_path, unit)?;
                let _ = std::process::Command::new("systemctl")
                    .args(["--user", "daemon-reload"])
                    .output();
                let _ = std::process::Command::new("loginctl")
                    .args(["enable-linger"])
                    .output();
                println!("Service registered on Linux");
            }
        }
        ServiceAction::Start => {
            #[cfg(windows)]
            { let _ = std::process::Command::new("sc").args(["start", service_name]).output(); }
            #[cfg(target_os = "macos")]
            { let _ = std::process::Command::new("launchctl").args(["start", "com.universalsearch.daemon"]).output(); }
            #[cfg(target_os = "linux")]
            { let _ = std::process::Command::new("systemctl").args(["--user", "start", "universal-search.service"]).output(); }
            println!("Service start requested");
        }
        ServiceAction::Stop => {
            #[cfg(windows)]
            { let _ = std::process::Command::new("sc").args(["stop", service_name]).output(); }
            #[cfg(target_os = "macos")]
            { let _ = std::process::Command::new("launchctl").args(["stop", "com.universalsearch.daemon"]).output(); }
            #[cfg(target_os = "linux")]
            { let _ = std::process::Command::new("systemctl").args(["--user", "stop", "universal-search.service"]).output(); }
            println!("Service stop requested");
        }
        ServiceAction::Status => {
            #[cfg(windows)]
            {
                let output = std::process::Command::new("sc").args(["query", service_name]).output()?;
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
            #[cfg(target_os = "macos")]
            {
                let output = std::process::Command::new("launchctl").args(["list", "com.universalsearch.daemon"]).output()?;
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
            #[cfg(target_os = "linux")]
            {
                let output = std::process::Command::new("systemctl").args(["--user", "is-active", "universal-search.service"]).output()?;
                println!("Service state: {}", String::from_utf8_lossy(&output.stdout));
            }
        }
        ServiceAction::Uninstall => {
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("sc").args(["stop", service_name]).output();
                let output = std::process::Command::new("sc").args(["delete", service_name]).output()?;
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
            #[cfg(target_os = "macos")]
            {
                let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
                let plist_path = home.join("Library").join("LaunchAgents").join("com.universalsearch.daemon.plist");
                let _ = std::process::Command::new("launchctl").args(["unload", "-w"]).arg(&plist_path).output();
                let _ = std::fs::remove_file(&plist_path);
                println!("Service uninstalled");
            }
            #[cfg(target_os = "linux")]
            {
                let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
                let unit_path = home.join(".config").join("systemd").join("user").join("universal-search.service");
                let _ = std::process::Command::new("systemctl").args(["--user", "stop", "universal-search.service"]).output();
                let _ = std::process::Command::new("systemctl").args(["--user", "disable", "universal-search.service"]).output();
                let _ = std::fs::remove_file(&unit_path);
                let _ = std::process::Command::new("systemctl").args(["--user", "daemon-reload"]).output();
                println!("Service uninstalled");
            }
        }
    }
    Ok(())
}

fn convert_paths(to: &str) -> anyhow::Result<()> {
    let mut config = config::Config::load_default()?;
    match to {
        "absolute" => {
            config.make_paths_absolute()?;
            config.save(&config::Config::default_config_path()?)?;
            println!("Paths converted to absolute");
        }
        "relative" => {
            config.make_paths_relative()?;
            config.save(&config::Config::default_config_path()?)?;
            println!("Paths converted to relative");
        }
        _ => {
            eprintln!("Use --to absolute or --to relative");
        }
    }
    Ok(())
}
