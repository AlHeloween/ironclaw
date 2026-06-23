---
title: "Universal Search Service"
description: "Unified search service: web search (crw-server → SearXNG), URL scraping, code search (Sourcegraph), AI-powered autonomous research (Claude Agent with prompt caching), hybrid search"
category: service
status: production
reproduce: "pwsh universal_search/build.ps1 && curl http://127.0.0.1:3005/health"
components:
  - universal_search/src/main.rs
  - universal_search/src/service.rs
  - universal_search/src/config.rs
  - universal_search/src/hybrid.rs
  - universal_search/src/web/firecrawl.rs
  - universal_search/src/web/sourcegraph.rs
  - universal_search/src/web/context.rs
  - universal_search/src/web/fetch.rs
  - universal_search/src/web/browser_fetch.rs
  - universal_search/src/ring_log.rs
  - universal_search/config.jsonc.template
  - universal_search/Cargo.toml
last_verified: 2026-06-14
---

# Universal Search Service

Unified search service combining web search (crw-server → SearXNG), URL scraping, code search (Sourcegraph GraphQL API), AI-powered autonomous research (Claude agent loop), and hybrid search.

Binds a single HTTP server (default `127.0.0.1:3005`). Web search uses:
- **crw-server** (port 3000): speaks Firecrawl API, proxies search to SearXNG
- **SearXNG** (port 3434): local Python meta-search engine → Google, Bing, Wikipedia

---

## File Layout

```
ironclaw/universal_search/
├── src/                      # Rust source
│   ├── main.rs               # CLI entry point
│   ├── service.rs            # HTTP service + agent loop + Claude integration
│   ├── config.rs             # Configuration (Agent, Firecrawl, Sourcegraph)
│   ├── web/
│   │   ├── firecrawl.rs      # Web search via crw-server (Firecrawl API)
│   │   ├── context.rs        # URL scraping via crw-server
│   │   ├── sourcegraph.rs    # Code search via Sourcegraph GraphQL
│   │   ├── fetch.rs          # Direct HTTPS fetch
│   │   └── browser_fetch.rs  # Playwright browser fetch
│   ├── hybrid.rs             # Hybrid search (web + Sourcegraph)
│   ├── ring_log.rs           # 10KB circular log buffer
│   └── bootstrap.rs          # Legacy Firecrawl bootstrap (disabled — source: "crw-server")
├── tests/                    # Rust unit tests + Python integration tests
├── config.jsonc.template     # TRACKED — reference template
├── config.jsonc              # GITIGNORED — active config with credentials
├── Cargo.toml                # Crate manifest
├── README.md                 # This file
└── AGENT_GUIDE.md            # Agent endpoint usage guide
```

---

## reproduce:

```bash
# 1. Copy template and fill in credentials (do once)
cp universal_search/config.jsonc.template universal_search/config.jsonc
notepad universal_search/config.jsonc
# Fill in: agent.anthropic_api_key, agent.anthropic_base_url

# 2. Build (produces target/release/)
cd universal_search && cargo build --release

# 3. Start dependent services (all NSSM-managed)
sc.exe start crw-server
sc.exe start searxng
sc.exe start rsedis
sc.exe start chromium-debug

# 4. Start universal-search
sc.exe start universal-search

# 5. Verify
curl http://127.0.0.1:3005/health
# => {"status":"healthy","service":"universal-search-service"}

curl -X POST http://127.0.0.1:3005/web/search \
  -H "Content-Type: application/json" \
  -d "{\"query\":\"rust programming\",\"count\":2}"
# => {"query":"rust programming","mode":"search","result_count":2,"results":[...]}

# 6. Test agent (requires Claude API key in config)
curl -X POST http://127.0.0.1:3005/agent \
  -H "Content-Type: application/json" \
  -d "{\"query\":\"what time is it now?\",\"max_turns\":3}"
```

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                Universal Search Service (3005)                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │    Agent     │  │  Web Search  │  │ Code Search  │           │
│  │  (Claude)    │  │  (Firecrawl  │  │ (Sourcegraph)│           │
│  │ + prompt     │  │   API via    │  │   GraphQL    │           │
│  │  caching)    │  │  crw-server) │  │              │           │
│  └──────┬───────┘  └──────┬───────┘  └──────────────┘           │
│         │                 │                                      │
└─────────┼─────────────────┼──────────────────────────────────────┘
          │                 │
          ▼                 ▼
┌─────────────────┐  ┌─────────────────────────────────────────┐
│ Anthropic API   │  │ crw-server (3000) — Rust scraper         │
│ (external HTTPS)│  │   /v1/search  → SearXNG (3434)           │
└─────────────────┘  │   /v1/scrape  → Playwright CDP (9222)    │
                     └──────────────┬──────────────────────────┘
                                    │
                                    ▼
                     ┌──────────────────────────────┐
                     │ SearXNG (3434) — Python       │
                     │   → Google, Bing, Wikipedia   │
                     │   cache → rsedis (6379)       │
                     └──────────────────────────────┘
```

The service **does not implement a local code index**. All code search goes through Sourcegraph's public GraphQL API.

---

## Dependencies (external services)

| Service | Port | Role | Managed by |
|---------|:---:|------|------------|
| crw-server | 3000 | Scraper: search + scrape via Playwright CDP | NSSM |
| SearXNG | 3434 | Meta-search engine → Google, Bing, Wikipedia | NSSM |
| rsedis | 6379 | Redis-compatible cache for SearXNG | NSSM |
| chromium-debug | 9222 | Chromium CDP for browser-based scraping | NSSM |
| websurfx | 3008 | User-facing web search UI (optional) | NSSM |

### Build prerequisites

| Dependency | Required For |
|-----------|-------------|
| Rust 1.92+ | Building the universal-search binary |
| Python 3.11+ | SearXNG runtime (if running locally) |
| NSSM | Windows service management |

---

## Quick Start

### Build

```powershell
cd universal_search
cargo build --release
```

Binary at `target/release/universal-search-service.exe` (9.7 MB).

### First-time config

```batch
copy universal_search\config.jsonc.template universal_search\config.jsonc
notepad universal_search\config.jsonc
:: Fill in agent credentials (Anthropic API key + base URL)
```

### Run

```batch
.\target\release\universal-search-service.exe run
```

Or as NSSM service (recommended):

```batch
nssm install universal-search .\target\release\universal-search-service.exe
nssm set universal-search AppDirectory .\target\release
nssm start universal-search
```

### All services (NSSM)

```batch
sc.exe query crw-server searxng rsedis chromium-debug universal-search
```

---

## CLI Commands

```
universal-search-service.exe [OPTIONS] [COMMAND]

Options:
  -c, --config PATH  Path to config file (default: config.jsonc next to binary)
  -p, --port PORT    Override service port
  -b, --bind ADDR    Override bind address
  --log-level LEVEL  trace | debug | info | warn | error (default: info)
  --log-file PATH    Write logs to file (1MB circular buffer)

Commands:
  run               Run in foreground (default)
  start             Start as daemon (background)
  stop              Stop the running service
  status [--json]   Check service health
  diag              Run full diagnostics (config + bootstrap)
  config [--create] Show or create configuration
  bootstrap         Run bootstrap only (check deps, create DB)
  service install   Register as OS service
  service start     Start OS service
  service stop      Stop OS service
  service status    Check OS service status
  service uninstall Remove OS service
  logs [-n N] [-f]  View circular log buffer (tail)
  convert-paths     Convert config paths between relative/absolute
```

---

## API Reference

### Health & Status

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `GET` | `/status` | Service status (agent enabled, web_search status) |

### Agent (AI-Powered Research)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/agent` | Start agent job |
| `GET` | `/agent/{id}` | Get job status |
| `DELETE` | `/agent/{id}` | Cancel job |

### Direct Search

| Method | Endpoint | Backend |
|--------|----------|---------|
| `POST` | `/web/search` | crw-server `/v1/search` → SearXNG → Google |
| `POST` | `/web/context` | crw-server `/v1/scrape` → Playwright CDP |
| `POST` | `/web/sourcegraph` | Sourcegraph GraphQL API |
| `POST` | `/web/fetch` | Direct HTTPS fetch |
| `POST` | `/web/browser` | Playwright browser fetch |
| `POST` | `/hybrid` | Sourcegraph + Web (merged) |

---

## Configuration

Config file: `config.jsonc` next to the binary (or set `UNIVERSAL_SEARCH_CONFIG` env var).

```jsonc
{
  "service": { "port": 3005, "bind_address": "127.0.0.1" },
  "agent": {
    "enabled": true,
    "model": "claude-sonnet-4-20250514",
    "max_turns": 5,
    "prompt_caching": true,
    "anthropic_api_key": "your-api-key",
    "anthropic_base_url": "https://your-proxy"
  },
  "web_search": {
    "firecrawl": {
      "api_url": "http://localhost:3000",
      "source": "crw-server"
    },
    "sourcegraph": {
      "api_url": "https://sourcegraph.com/.api/graphql",
      "access_token": null
    }
  }
}
```

---

## Testing

```bash
# Rust unit tests
cd universal_search && cargo test
# => 29 passed (1 pre-existing ring_log skip)

# Integration tests (mockito)
cargo test --test test_web_search --test test_sourcegraph
# => 11 passed
```

---

## Troubleshooting

### Web search returns empty results
Ensure crw-server and SearXNG are running:
```powershell
sc.exe query crw-server searxng
curl http://127.0.0.1:3000/v1/search -X POST -H "Content-Type: application/json" -d '{"query":"test","limit":2}'
curl "http://127.0.0.1:3434/search?q=test&format=json"
```

### Port already in use
```powershell
netstat -ano | findstr :3005
taskkill /PID <PID> /F
```

### Agent returns "Rate limited"
Wait `rate_limit_seconds` between requests, or increase in config.

### Agent job stuck in "processing"
Check Claude API is reachable. Jobs auto-expire after `job_ttl_seconds`.

### SearXNG not starting
Check Python 3.11+ available, deps installed, settings.yml at correct path.
```powershell
python -m searx.webapp  # test foreground
```

---

## References

- [SearXNG Documentation](https://docs.searxng.org/)
- [Sourcegraph API](https://docs.sourcegraph.com/api)
- [Anthropic Messages API](https://docs.anthropic.com/en/api/messages)
- [Anthropic Prompt Caching](https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching)
- [IronClaw web-search extension](docs/extensions/web-search.md)

---

> **Last verified:** 2026-06-14 — SearXNG + crw-server pipeline, agent prompt caching, all 6 NSSM services healthy, universalsearch tool returns live results.
