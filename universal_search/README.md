---
title: "Universal Search Service"
description: "Unified search service: web search (Firecrawl), code search (Sourcegraph), AI-powered autonomous research (Claude Agent), and hybrid search"
category: service
status: production
components:
  - universal_search/src/main.rs
  - universal_search/src/service.rs
  - universal_search/src/bootstrap.rs
  - universal_search/src/config.rs
  - universal_search/src/hybrid.rs
  - universal_search/src/web/firecrawl.rs
  - universal_search/src/web/sourcegraph.rs
  - universal_search/src/web/context.rs
  - universal_search/src/ring_log.rs
  - universal_search/config.jsonc.template
  - universal_search/Cargo.toml
---

# Universal Search Service

Unified search service combining web search (Firecrawl), URL scraping, code search (Sourcegraph GraphQL API), AI-powered autonomous research (Claude agent loop), and hybrid search.

Binds a single HTTP server (default `127.0.0.1:3005`) and proxies search/agent requests to Firecrawl (default `localhost:3002`) and Sourcegraph.

---

## File Layout

```
ironclaw/
├── .gitignore                    # Protects config and build output from git
├── dist/                         # Project-wide distribution output (gitignored)
│   └── universal-search/         # Canonical runnable output for this service
│       ├── universal-search-service.exe
│       ├── config.jsonc          ← YOUR ACTIVE CONFIG with credentials
│       ├── run.bat
│       ├── install.bat
│       └── AGENT_GUIDE.md
│
└── universal_search/             # Source crate for this service
    ├── config.jsonc              # GITIGNORED — active config with credentials.
    │                             # Binary does NOT read this file. It only exists
    │                             # here so you can edit it, then build.ps1 copies
    │                             # it to dist/universal-search/ where the binary
    │                             # reads it from next to itself.
    │                             #
    │                             # Create from config.jsonc.template once.
    │                             # Never commit to git.
    │
    ├── config.jsonc.template     # TRACKED — reference template. No credentials.
    │                             # Copy this to config.jsonc and fill in your keys.
    │
    ├── build.ps1                 # TRACKED — single canonical build script.
    │                             # 1. cargo build
    │                             # 2. Copies binary → dist/universal-search/
    │                             # 3. Copies config.jsonc → dist/universal-search/
    │                             # 4. Creates run.bat + install.bat
    │
    ├── src/                      # Rust source (main.rs, service.rs, bootstrap.rs, …)
    ├── tests/                    # Rust unit tests + Python integration tests
    ├── Cargo.toml                # Crate manifest
    ├── README.md                 # This file
    └── AGENT_GUIDE.md            # Agent endpoint usage guide
```

### What is tracked vs gitignored

| File / Directory | Git | Contains | Purpose |
|:---|---:|---|---|
| `config.jsonc.template` | tracked | Placeholders only | Reference template — copy once |
| `config.jsonc` | **gitignored** | Credentials (API keys, passwords) | Your active config — edit here, never commit |
| `dist/universal-search/config.jsonc` | **gitignored** | Copy of your config | Read by binary at runtime |
| `dist/universal-search/` | **gitignored** | Binary, config, scripts | Canonical runnable output — sole output of `build.ps1` |
| `target/` | **gitignored** | Cargo build artifacts | Transient |
| Everything else | tracked | Source, docs, templates | Committed |

**One config, two locations (edit one, build copies):**

```
universal_search/config.jsonc   ← YOU EDIT THIS (gitignored)
         │  build.ps1 copies
         ▼
dist/universal-search/config.jsonc  ← BINARY READS THIS (gitignored)
```

The binary finds `config.jsonc` next to itself in `dist/universal-search/` — no `--config` flag needed.

---

## reproduce:

```bash
# 1. Copy template and fill in credentials (do once)
cp universal_search/config.jsonc.template universal_search/config.jsonc
notepad universal_search/config.jsonc
# Fill in: agent.anthropic_api_key, agent.anthropic_base_url,
#          web_search.firecrawl.postgres.password

# 2. Build (produces dist/universal-search/)
pwsh universal_search/build.ps1

# 3. Start PostgreSQL (port 5432) and Redis (port 6379) — Firecrawl dependencies

# 4. Run diagnostics (creates firecrawl DB if missing)
.\dist\universal-search\universal-search-service.exe diag

# 5. Start service
.\dist\universal-search\run.bat

# 6. Verify
curl http://127.0.0.1:3005/health
# => {"status":"healthy","service":"universal-search-service"}

curl -X POST http://127.0.0.1:3005/web/search \
  -H "Content-Type: application/json" \
  -d "{\"query\":\"hello world\",\"count\":2}"
# => {"query":"hello world","mode":"search","result_count":2,"results":[...]}

# 7. Test agent (requires Claude API)
curl -X POST http://127.0.0.1:3005/agent \
  -H "Content-Type: application/json" \
  -d "{\"query\":\"what time is it now?\",\"max_turns\":3}"
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│              Universal Search Service (3005)                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Agent     │  │  Web Search │  │ Code Search │         │
│  │ (Claude)    │  │ (Firecrawl) │  │(Sourcegraph)│         │
│  │             │  │  + Scrape   │  │  GraphQL    │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Hybrid (Sourcegraph + Web)             │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │     Bootstrap (PG check, DB create, git clone,      │    │
│  │               pnpm install, Firecrawl start)        │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                               │
               ┌───────────────┼───────────────┐
               ▼               ▼               ▼
        ┌────────────┐  ┌────────────┐  ┌────────────┐
        │ PostgreSQL │  │   Redis    │  │ Sourcegraph│
        │  (5432)    │  │   (6379)   │  │  (public)  │
        └────────────┘  └────────────┘  └────────────┘
               │
               ▼
        ┌────────────┐
        │  Firecrawl │
        │  (3002)    │
        └────────────┘
```

The service **does not implement a local code index** (Tantivy on port 3004 is a planned feature). All code search goes through Sourcegraph's public GraphQL API.

---

## Prerequisites

| Dependency | Required For | Default Port |
|-----------|-------------|:---:|
| PostgreSQL | Firecrawl job storage | 5432 |
| Redis | Firecrawl caching | 6379 |
| Node.js v24+ | Firecrawl server runtime | — |
| pnpm | Firecrawl dependency install | — |
| Rust 1.92+ | Building the service | — |
| psql | Bootstrap DB creation | (PATH) |
| git | Bootstrap: clone Firecrawl repo | — |
| NSSM | Windows service installation (optional) | — |

---

## Quick Start

### Build

```powershell
pwsh universal_search/build.ps1
```

Produces `dist/universal-search/` containing:
- `universal-search-service.exe` — the binary
- `config.jsonc` — your active config (copied from `universal_search/config.jsonc`)
- `run.bat` — foreground run script
- `install.bat` — Windows service installer
- `AGENT_GUIDE.md` — agent usage guide

### First-time config

```batch
copy universal_search\config.jsonc.template universal_search\config.jsonc
notepad universal_search\config.jsonc
:: Fill in credentials, then run build.ps1
```

### Run

```batch
cd dist\universal-search
.\run.bat
```

### Install as Windows Service

```batch
cd dist\universal-search
.\install.bat
```

### Default Ports

| Service | Port | Purpose |
|---------|:---:|---------|
| universal_search | 3005 | HTTP API for all endpoints |
| Firecrawl | 3002 | Web scraping and extraction |

---

## Bootstrap (Auto-setup)

On startup with `source: "local"`, the service runs a bootstrap sequence:

1. **Check PostgreSQL** — TCP port 5432 must be reachable
2. **Ensure Firecrawl database** — If the `firecrawl` database doesn't exist in PostgreSQL, creates it via `psql` using configured credentials. Searches common psql install paths and falls back to PATH.
3. **Check Redis** — TCP port 6379 must be reachable
4. **Clone Firecrawl** — If `bootstrap.auto_clone_firecrawl: true` and the repo is missing, runs `git clone --depth=1 https://github.com/firecrawl/firecrawl.git`
5. **Install deps** — If `bootstrap.auto_install_deps: true`, runs `pnpm install` in `firecrawl/apps/api`
6. **Start Firecrawl** — If `firecrawl.auto_start: true`, launches Firecrawl (detached, `USE_GO_MARKDOWN_PARSER=false`)

### Database Auto-Creation

If PostgreSQL is reachable but the `firecrawl` database is missing, the service finds `psql` and runs:

```sql
CREATE DATABASE "firecrawl";
```

This uses the credentials from `web_search.firecrawl.postgres` in config.

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
| `POST` | `/web/search` | Firecrawl `/v1/search` |
| `POST` | `/web/context` | Firecrawl `/v1/scrape` |
| `POST` | `/web/sourcegraph` | Sourcegraph GraphQL API |
| `POST` | `/hybrid` | Sourcegraph + Firecrawl (merged) |

---

## Configuration

Config file: `universal_search/config.jsonc` (edit here), copied by `build.ps1` to `dist/universal-search/config.jsonc` (read by binary).

```jsonc
{
  "service": { "port": 3005, "bind_address": "127.0.0.1" },
  "agent": {
    "enabled": true,
    "model": "claude-sonnet-4-20250514",
    "anthropic_api_key": "your-api-key",
    "anthropic_base_url": "https://your-proxy/v1"
  },
  "web_search": {
    "firecrawl": {
      "api_url": "http://localhost:3002",
      "source": "local",
      "auto_start": false,
      "postgres": {
        "host": "localhost", "port": 5432,
        "username": "postgres", "password": "your-password",
        "database": "firecrawl"
      },
      "redis": { "host": "localhost", "port": 6379 }
    },
    "sourcegraph": {
      "api_url": "https://sourcegraph.com/.api/graphql",
      "access_token": null
    }
  },
  "bootstrap": {
    "auto_clone_firecrawl": false,
    "auto_install_deps": false
  }
}
```

---

## Service Management (Windows / NSSM)

```batch
:: Install (from dist/universal-search/)
.\install.bat

:: Control
nssm start universal-search
nssm stop universal-search
nssm status universal-search

:: Remove
nssm remove universal-search confirm
```

---

## Testing

```bash
# Rust unit tests (mockito)
cargo test -p universal-search-service

# Python integration tests
cd universal_search/tests
pytest test_integration.py -v

# Smoke test
pwsh universal_search/test.ps1
```

---

## Troubleshooting

### Port already in use
```powershell
netstat -ano | findstr :3005
taskkill /PID <PID> /F
```

### Firecrawl connection refused
Ensure Firecrawl is running:
```bash
curl http://localhost:3002/
# => {"message":"Firecrawl API","documentation_url":"https://docs.firecrawl.dev"}
```

### Agent returns "Rate limited"
Wait `rate_limit_seconds` between requests, or increase in config.

### Agent job stuck in "processing"
Check Claude API is reachable. Jobs auto-expire after `job_ttl_seconds`.

### Bootstrap warnings
```bash
.\dist\universal-search\universal-search-service.exe diag
```
Common causes: PostgreSQL/Redis not running, `psql` not in PATH, `git`/`pnpm` missing.

---

## References

- [Firecrawl Docs](https://docs.firecrawl.dev)
- [Sourcegraph API](https://docs.sourcegraph.com/api)
- [Anthropic Messages API](https://docs.anthropic.com/en/api/messages)
- [IronClaw web-search extension](docs/extensions/web-search.md)

---

> **Last verified:** 2026-05-19 — build passes, DB auto-creation tested, config default fix verified.
