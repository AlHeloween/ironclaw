//! Configuration for the Local Code Search Service.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalCodeSearchConfig {
    #[serde(default)]
    pub service: ServiceConfig,

    #[serde(default)]
    pub indexes: Vec<IndexConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct ServiceConfig {
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    #[serde(default = "default_true")]
    pub watch_enabled: bool,

    #[serde(default = "default_debounce_ms")]
    pub watch_debounce_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    pub name: String,
    pub path: String,

    #[serde(default = "default_languages")]
    pub languages: Vec<String>,

    #[serde(default)]
    pub include: Vec<String>,

    #[serde(default = "default_excludes")]
    pub exclude: Vec<String>,

    #[serde(default = "default_true")]
    pub symbols_enabled: bool,

    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_port() -> u16 {
    3004
}
fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}
fn default_true() -> bool {
    true
}
fn default_debounce_ms() -> u64 {
    500
}
fn default_languages() -> Vec<String> {
    vec!["all".to_string()]
}
fn default_max_file_size() -> u64 {
    1024 * 1024
}

fn default_excludes() -> Vec<String> {
    vec![
        "target/".to_string(),
        "node_modules/".to_string(),
        ".git/".to_string(),
        "dist/".to_string(),
        "build/".to_string(),
        ".next/".to_string(),
        "venv/".to_string(),
        "__pycache__/".to_string(),
        ".mypy_cache/".to_string(),
        ".pytest_cache/".to_string(),
        "*.pyc".to_string(),
        "*.so".to_string(),
        "*.dll".to_string(),
        "*.dylib".to_string(),
        ".DS_Store".to_string(),
    ]
}

impl LocalCodeSearchConfig {
    pub fn load(path: &Path) -> Result<Self, anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_jsonc::from_str(&content)?;
        Ok(config)
    }

    pub fn load_or_default() -> Self {
        let default_path = dirs::home_dir()
            .map(|mut p| {
                p.push(".ironclaw/local-code-search.jsonc");
                p
            })
            .expect("Could not determine home directory");

        Self::load(&default_path).unwrap_or_default()
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.service.port = port;
        self
    }

    pub fn with_bind_address(mut self, bind: String) -> Self {
        self.service.bind_address = bind;
        self
    }
}

impl Default for LocalCodeSearchConfig {
    fn default() -> Self {
        Self {
            service: ServiceConfig {
                port: default_port(),
                bind_address: default_bind_address(),
                watch_enabled: true,
                watch_debounce_ms: default_debounce_ms(),
            },
            indexes: Vec::new(),
        }
    }
}
