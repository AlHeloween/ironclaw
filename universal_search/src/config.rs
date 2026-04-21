//! Unified configuration for the universal search service.
//!
//! Supports both relative and absolute paths. Paths are resolved
//! relative to the executable directory for portability.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{env, fs};

/// Root configuration structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub service: ServiceConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub web_search: WebSearchConfig,
    #[serde(default)]
    pub bootstrap: BootstrapConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind_address: String,
}

fn default_port() -> u16 {
    3005
}

fn default_bind() -> String {
    "127.0.0.1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebSearchConfig {
    #[serde(default)]
    pub firecrawl: FirecrawlConfig,
    #[serde(default)]
    pub sourcegraph: SourcegraphConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FirecrawlConfig {
    #[serde(default = "default_firecrawl_url")]
    pub api_url: String,
    pub api_key: Option<String>,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default = "default_true")]
    pub auto_start: bool,
    #[serde(default = "default_repo_path")]
    pub repo_path: String,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub postgres: Option<PostgresConfig>,
    #[serde(default)]
    pub redis: Option<RedisConfig>,
}

fn default_firecrawl_url() -> String {
    "http://localhost:3002".to_string()
}

fn default_source() -> String {
    "local".to_string()
}

fn default_repo_path() -> String {
    "./firecrawl".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    #[serde(default = "default_pg_host")]
    pub host: String,
    #[serde(default = "default_pg_port")]
    pub port: u16,
    #[serde(default = "default_pg_user")]
    pub username: String,
    pub password: Option<String>,
    #[serde(default = "default_pg_db")]
    pub database: String,
}

fn default_pg_host() -> String {
    "localhost".to_string()
}

fn default_pg_port() -> u16 {
    5432
}

fn default_pg_user() -> String {
    "postgres".to_string()
}

fn default_pg_db() -> String {
    "firecrawl".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    #[serde(default = "default_redis_host")]
    pub host: String,
    #[serde(default = "default_redis_port")]
    pub port: u16,
    pub password: Option<String>,
}

fn default_redis_host() -> String {
    "localhost".to_string()
}

fn default_redis_port() -> u16 {
    6379
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcegraphConfig {
    #[serde(default = "default_sourcegraph_url")]
    pub api_url: String,
    pub access_token: Option<String>,
}

fn default_sourcegraph_url() -> String {
    "https://sourcegraph.com/.api/graphql".to_string()
}

impl Default for SourcegraphConfig {
    fn default() -> Self {
        Self {
            api_url: default_sourcegraph_url(),
            access_token: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    #[serde(default = "default_true")]
    pub auto_clone_firecrawl: bool,
    #[serde(default = "default_true")]
    pub auto_install_deps: bool,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            auto_clone_firecrawl: true,
            auto_install_deps: true,
        }
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            port: 3005,
            bind_address: "127.0.0.1".to_string(),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    #[serde(default = "default_agent_model")]
    pub model: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default = "default_rate_limit_seconds")]
    pub rate_limit_seconds: u64,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    #[serde(default = "default_job_ttl_seconds")]
    pub job_ttl_seconds: u64,
    #[serde(default = "default_max_input_tokens")]
    pub max_input_tokens: u32,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u32,
    #[serde(default = "default_retry_max_attempts")]
    pub retry_max_attempts: u32,
    #[serde(default = "default_retry_delay_seconds")]
    pub retry_delay_seconds: u64,
    #[serde(default)]
    pub anthropic_api_key: Option<String>,
    #[serde(default)]
    pub anthropic_base_url: Option<String>,
}

fn default_max_turns() -> u32 {
    5
}
fn default_agent_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}
pub(crate) fn default_agent_system_prompt() -> String {
    "You are a research assistant with powerful web tools. Your job is to find accurate, current information by actively using your available tools.\n\nCRITICAL: When the user asks about anything on the internet, current events, facts, documentation, or real-world information, you MUST use the search tool. Do NOT say you can't access real-time data - you CAN, using the search tool.\n\nTIME AWARENESS: You should be aware of the current time frame. If the user asks about the current time, date, or any time-sensitive information, use the search tool to find the current time from reliable sources like time.is or worldclock.com. Do NOT say you don't have access to real-time information - you DO have access via the search tool.\n\nAvailable tools:\n- search(query): Search the web and get scraped content from top results. USE THIS for any internet research, including current time.\n- scrape(url): Extract full content from a specific URL (HTML, PDF, documents)\n- crawl(url, limit): Crawl multiple pages from a seed URL\n- map(url, search): Discover all URLs on a domain, filtered by keyword\n\nWorkflow:\n1. If the question involves ANY external information (internet, facts, documentation, current events, current time) -> ALWAYS use search first\n2. Use scrape to get detailed content from the most relevant URLs found by search\n3. Cross-reference multiple sources when the topic is complex\n4. Cite your sources (URLs) in the final answer\n5. If search returns no relevant results, try different search terms\n6. Only after exhausting all tool options, acknowledge limitations\n\nImportant:\n- You have full access to current web content via the search tool\n- Use search proactively - don't wait for the user to ask you to search\n- Respond in the same language the user used\n- Be thorough but efficient with your tool usage\n- No source constraints - provide the most relevant information available".to_string()
}
fn default_rate_limit_seconds() -> u64 {
    10
}
fn default_max_concurrent() -> usize {
    10
}
fn default_job_ttl_seconds() -> u64 {
    3600
}
fn default_max_input_tokens() -> u32 {
    200000
}
fn default_max_output_tokens() -> u32 {
    100000
}
fn default_retry_max_attempts() -> u32 {
    5
}
fn default_retry_delay_seconds() -> u64 {
    10
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_turns: default_max_turns(),
            model: default_agent_model(),
            system_prompt: None,
            rate_limit_seconds: default_rate_limit_seconds(),
            max_concurrent: default_max_concurrent(),
            job_ttl_seconds: default_job_ttl_seconds(),
            max_input_tokens: default_max_input_tokens(),
            max_output_tokens: default_max_output_tokens(),
            retry_max_attempts: default_retry_max_attempts(),
            retry_delay_seconds: default_retry_delay_seconds(),
            anthropic_api_key: None,
            anthropic_base_url: None,
        }
    }
}

/// Resolve a path relative to the executable directory.
pub fn exe_dir() -> anyhow::Result<PathBuf> {
    if let Ok(dir) = env::var("UNIVERSAL_SEARCH_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let exe = env::current_exe()?;
    Ok(exe.parent().unwrap_or(Path::new(".")).to_path_buf())
}

/// Resolve a potentially relative path to an absolute path.
pub fn resolve_path(base: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    base.join(p)
}

/// Convert an absolute path to relative (if possible).
pub fn make_relative(base: &Path, path: &Path) -> String {
    if !path.is_absolute() {
        return path.display().to_string();
    }
    if let Ok(relative) = path.strip_prefix(base) {
        if relative.as_os_str().is_empty() {
            return ".".to_string();
        }
        return relative.display().to_string();
    }
    path.display().to_string()
}

impl Config {
    /// Load configuration from a file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_jsonc::from_str(&content)?;
        Ok(config)
    }

    /// Load from the default config location.
    pub fn load_default() -> anyhow::Result<Self> {
        if let Ok(path) = env::var("UNIVERSAL_SEARCH_CONFIG") {
            return Self::load(Path::new(&path));
        }
        let base = exe_dir()?;
        let config_path = base.join("config.jsonc");
        if config_path.exists() {
            return Self::load(&config_path);
        }
        Ok(Config::default())
    }

    /// Get the default config file path.
    pub fn default_config_path() -> anyhow::Result<PathBuf> {
        Ok(exe_dir()?.join("config.jsonc"))
    }

    /// Write default config to a path.
    pub fn write_default(path: &Path) -> anyhow::Result<()> {
        let config = Config::default();
        let content = serde_jsonc::to_string_pretty(&config)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Resolve all paths in the config to absolute.
    pub fn resolve_paths(&mut self) -> anyhow::Result<()> {
        let base = exe_dir()?;
        self.web_search.firecrawl.repo_path =
            resolve_path(&base, &self.web_search.firecrawl.repo_path)
                .display()
                .to_string();
        Ok(())
    }

    /// Convert all paths to relative.
    pub fn make_paths_relative(&mut self) -> anyhow::Result<()> {
        let base = exe_dir()?;
        self.web_search.firecrawl.repo_path =
            make_relative(&base, Path::new(&self.web_search.firecrawl.repo_path));
        Ok(())
    }

    /// Convert all paths to absolute.
    pub fn make_paths_absolute(&mut self) -> anyhow::Result<()> {
        self.resolve_paths()
    }

    /// Save config to a file.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = serde_jsonc::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Get the Firecrawl repo path (resolved).
    pub fn firecrawl_repo_path(&self) -> anyhow::Result<PathBuf> {
        let base = exe_dir()?;
        Ok(resolve_path(&base, &self.web_search.firecrawl.repo_path))
    }

    /// Display config with secrets masked.
    pub fn masked_display(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("service.port: {}\n", self.service.port));
        s.push_str(&format!(
            "service.bind_address: {}\n",
            self.service.bind_address
        ));
        s.push_str(&format!("agent.enabled: {}\n", self.agent.enabled));
        s.push_str(&format!("agent.max_turns: {}\n", self.agent.max_turns));
        s.push_str(&format!("agent.model: {}\n", self.agent.model));
        s.push_str(&format!(
            "web_search.firecrawl.api_url: {}\n",
            self.web_search.firecrawl.api_url
        ));
        s.push_str(&format!(
            "web_search.firecrawl.api_key: {}\n",
            if self.web_search.firecrawl.api_key.is_some() {
                "***"
            } else {
                "(not set)"
            }
        ));
        s.push_str(&format!(
            "web_search.sourcegraph.access_token: {}\n",
            if self.web_search.sourcegraph.access_token.is_some() {
                "***"
            } else {
                "(not set)"
            }
        ));
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_serializes() {
        let config = Config::default();
        let json = serde_jsonc::to_string_pretty(&config).unwrap();
        let parsed: Config = serde_jsonc::from_str(&json).unwrap();
        assert_eq!(parsed.service.port, 3005);
    }

    #[test]
    fn resolve_path_handles_relative() {
        let base = Path::new("/opt/universal_search");
        let resolved = resolve_path(base, "./indexes");
        assert_eq!(resolved, Path::new("/opt/universal_search/indexes"));
    }

    #[test]
    fn resolve_path_handles_absolute() {
        let base = Path::new("/opt/universal_search");
        let resolved = resolve_path(base, "/var/data");
        assert_eq!(resolved, Path::new("/var/data"));
    }
}
