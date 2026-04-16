//! Configuration for the Firecrawl Search Service.
//!
//! Loaded from `~/.ironclaw/firecrawl-search.jsonc` with sensible defaults.

use serde::{Deserialize, Serialize};
use std::path::Path;

const DEFAULT_PORT: u16 = 3005;
const DEFAULT_BIND: &str = "127.0.0.1";
const DEFAULT_FIRECRAWL_URL: &str = "http://localhost:3002";
const DEFAULT_LOCAL_SEARCH_URL: &str = "http://127.0.0.1:3004";
const SOURCEGRAPH_API_URL: &str = "https://sourcegraph.com/.api/graphql";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FirecrawlSearchConfig {
    pub service: ServiceConfig,
    pub firecrawl: FirecrawlConfig,
    pub sourcegraph: SourcegraphConfig,
    pub local_search: LocalSearchConfig,
}

impl Default for FirecrawlSearchConfig {
    fn default() -> Self {
        Self {
            service: ServiceConfig::default(),
            firecrawl: FirecrawlConfig::default(),
            sourcegraph: SourcegraphConfig::default(),
            local_search: LocalSearchConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServiceConfig {
    pub port: u16,
    pub bind_address: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            bind_address: DEFAULT_BIND.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FirecrawlConfig {
    pub api_url: String,
    pub api_key: Option<String>,
}

impl Default for FirecrawlConfig {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_FIRECRAWL_URL.to_string(),
            api_key: std::env::var("FIRECRAWL_API_KEY").ok(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SourcegraphConfig {
    pub api_url: String,
    pub access_token: Option<String>,
}

impl Default for SourcegraphConfig {
    fn default() -> Self {
        Self {
            api_url: SOURCEGRAPH_API_URL.to_string(),
            access_token: std::env::var("SOURCEGRAPH_ACCESS_TOKEN").ok(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalSearchConfig {
    pub url: String,
    pub enabled: bool,
}

impl Default for LocalSearchConfig {
    fn default() -> Self {
        Self {
            url: DEFAULT_LOCAL_SEARCH_URL.to_string(),
            enabled: true,
        }
    }
}

impl FirecrawlSearchConfig {
    pub fn load(path: &Path) -> Result<Self, anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_jsonc::from_str(&content)?;
        Ok(config)
    }

    pub fn load_default() -> Result<Self, anyhow::Error> {
        let config_path = dirs::home_dir()
            .map(|mut p| {
                p.push(".ironclaw/firecrawl-search.jsonc");
                p
            })
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        if config_path.exists() {
            Self::load(&config_path)
        } else {
            Ok(Self::default())
        }
    }
}
