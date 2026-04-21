# Universal Search Service

Unified search service combining web search (Firecrawl), code search (Sourcegraph), and AI-powered autonomous research (Claude Agent).

## Installation as Windows Service

### Prerequisites

- **NSSM** (Non-Sucking Service Manager) — Install with: `winget install nssm.nssm`
- **PostgreSQL** running on localhost:5432
- **Redis** running on localhost:6379

### Step 1: Build the Binary

```bash
cargo build --release -p universal-search-service
```

### Step 2: Prepare the Distribution Folder

```batch
mkdir universal_search\dist
copy target\release\universal-search-service.exe universal_search\dist\
copy universal_search\config.jsonc universal_search\dist\
```

### Step 3: Install Firecrawl Service (Dependency)

Firecrawl must be running before the Universal Search service starts:

```batch
universal_search\install_firecrawl_service.bat
```

This installs Firecrawl as a Windows service with:
- Auto-start on boot
- Below average process priority
- Automatic restart on failure (5 second delay)
- Port 3002

### Step 4: Install Universal Search Service

```batch
universal_search\install_service.bat
```

This installs the Universal Search service with:
- Auto-start on boot
- Below average process priority
- Automatic restart on failure (5 second delay)
- Port 3005

### Service Priority

Both services are installed with **SERVICE_NORMAL** priority (below average), ensuring they don't compete with interactive applications for CPU time.

### Managing Services

#### Windows (NSSM)

```batch
REM Check status
nssm status universal-search
nssm status Firecrawl

REM Stop
nssm stop universal-search
nssm stop Firecrawl

REM Start
nssm start universal-search
nssm start Firecrawl

REM Remove
nssm remove universal-search confirm
nssm remove Firecrawl confirm
```

#### Linux (systemd)

```bash
# Check status
sudo systemctl status universal-search
sudo systemctl status firecrawl

# Stop
sudo systemctl stop universal-search
sudo systemctl stop firecrawl

# Start
sudo systemctl start universal-search
sudo systemctl start firecrawl

# Remove
sudo systemctl disable --now universal-search
sudo systemctl disable --now firecrawl

# View logs
sudo journalctl -u universal-search -f
sudo journalctl -u firecrawl -f
```

### Viewing Logs

**Windows**: Services write to the Windows Event Log. Use Event Viewer or:
```powershell
Get-EventLog -LogName Application -Source nssm -Newest 50
```

**Linux**: Use journalctl:
```bash
sudo journalctl -u universal-search -f
sudo journalctl -u firecrawl -f
```

### Uninstall

**Windows**: Run `universal_search\uninstall_service.bat`

**Linux**: Run `sudo ./universal_search/uninstall_service.sh`

---

## Quick Start

### Prerequisites

- **PostgreSQL** (localhost:5432) — Firecrawl job storage
- **Redis** (localhost:6379) — Firecrawl caching
- **Node.js** v24+ — Firecrawl server runtime
- **pnpm** — Node.js package manager
- **Rust** 1.92+ — for building the service

### Build

```bash
cargo build --release -p universal-search-service
```

### Run

#### As Windows Service (Recommended)

```batch
# Copy binary and config to dist/
mkdir universal_search\dist
copy target\release\universal-search-service.exe universal_search\dist\
copy universal_search\config.jsonc universal_search\dist\

# Install and start the service
universal_search\install_service.bat
```

The service auto-starts on boot and restarts on failure.

#### Manual (Foreground)

```batch
# Copy config next to the binary
cp universal_search/config.jsonc target/release/config.jsonc

# Start the service
target/release/universal-search-service.exe run
```

Or from the dist folder:
```batch
universal_search\dist\run.bat
```

### Default Ports

| Service | Port | Purpose |
|---------|------|---------|
| universal_search (main) | 3005 | HTTP API for all search endpoints |
| Firecrawl | 3002 | Web scraping and AI extraction |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Universal Search (3005)                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Agent     │  │  Web Search │  │ Code Search │         │
│  │ (Claude)    │  │ (Firecrawl) │  │(Sourcegraph)│         │
│  │             │  │             │  │             │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
                               │
               ┌───────────────┼───────────────┐
               ▼               ▼               ▼
        ┌────────────┐  ┌────────────┐  ┌────────────┐
        │ PostgreSQL │  │   Redis    │  │ Sourcegraph│
        │  (5432)    │  │   (6379)   │  │    (API)   │
        └────────────┘  └────────────┘  └────────────┘
```

---

## API Reference

### Health Check

```
GET /health
```

### Status

```
GET /status
```

### Agent (AI-Powered Research)

The agent endpoint provides autonomous research capabilities using Claude and Firecrawl tools.

#### Start Agent Job

```
POST /agent
Content-Type: application/json

{
  "query": "what time now?",
  "max_turns": 5,
  "system_prompt": null,
  "model": null
}
```

#### Get Agent Status

```
GET /agent/{id}
```

#### Cancel Agent Job

```
DELETE /agent/{id}
```

### Web Search (Firecrawl)

```
POST /web/search
Content-Type: application/json

{
  "query": "rust async programming",
  "count": 5
}
```

### URL Scraping (Firecrawl)

```
POST /web/context
Content-Type: application/json

{
  "query": "https://example.com",
  "url": "https://example.com",
  "scrape_formats": "markdown",
  "only_main_content": true
}
```

### Sourcegraph Code Search

```
POST /web/sourcegraph
Content-Type: application/json

{
  "query": "lang:rust async fn",
  "count": 10
}
```

### Hybrid Search (Sourcegraph + Web)

```
POST /hybrid
Content-Type: application/json

{
  "query": "async handler",
  "count": 10
}
```

---

## Configuration

Config file: `config.jsonc` (placed next to the exe or at `~/.ironclaw/universal-search.jsonc`)

```jsonc
{
  "service": {
    "port": 3005,
    "bind_address": "127.0.0.1"
  },

  "agent": {
    "enabled": true,
    "max_turns": 5,
    "model": "claude-sonnet-4-20250514",
    "system_prompt": null,
    "rate_limit_seconds": 10,
    "max_concurrent": 10,
    "job_ttl_seconds": 3600,
    "max_input_tokens": 200000,
    "max_output_tokens": 100000,
    "retry_max_attempts": 5,
    "retry_delay_seconds": 10,
    "anthropic_api_key": "your-api-key-here",
    "anthropic_base_url": "https://your-anthropic-proxy/v1"
  },

  "web_search": {
    "firecrawl": {
      "api_url": "http://localhost:3002",
      "api_key": null,
      "source": "local",
      "auto_start": false,
      "repo_path": "../../universal_search/firecrawl",
      "postgres": {
        "host": "localhost",
        "port": 5432,
        "username": "postgres",
        "password": "1412",
        "database": "firecrawl"
      },
      "redis": {
        "host": "localhost",
        "port": 6379
      }
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

## Firecrawl Setup

Firecrawl is located at `universal_search/firecrawl/` and provides web scraping and AI-powered extraction.

### Start as Windows Service (Recommended)

```batch
universal_search\install_firecrawl_service.bat
```

This installs Firecrawl as a Windows service via NSSM. It auto-starts on boot.

### Manual Start

```batch
cd universal_search\firecrawl\apps\api
start_local.bat
```

### Environment Variables (for start_local.bat)

```batch
set ANTHROPIC_API_KEY=your_key_here
set ANTHROPIC_BASE_URL=your_anthropic_proxy_url
```

---

## Documentation

- **[Firecrawl API Reference](FIRECRAWL_API.md)** — Complete Firecrawl API documentation
- [Firecrawl Docs](https://docs.firecrawl.dev) — Official Firecrawl documentation
- [Sourcegraph API](https://docs.sourcegraph.com/api) — Sourcegraph GraphQL API

---

## Troubleshooting

### Port Already in Use

```
Error: listen EADDRINUSE: address already in use 0.0.0.0:3002
```

Kill the process using the port:

```powershell
netstat -ano | findstr :3002
taskkill /PID <PID> /F
```

### PostgreSQL Connection Failed

Ensure PostgreSQL is running on localhost:5432.

### Redis Connection Failed

Ensure Redis is running on localhost:6379:

```powershell
redis-cli PING
```

### Agent Endpoint Not Working

1. Ensure `anthropic_api_key` and `anthropic_base_url` are set in `config.jsonc`
2. Ensure Firecrawl is running on port 3002
3. Check the agent is enabled in config (`agent.enabled: true`)

---

## Development

### Project Structure

```
universal_search/
├── src/
│   ├── lib.rs              # Library entry point
│   ├── main.rs             # Binary entry point
│   ├── config.rs           # Configuration types
│   ├── service.rs          # HTTP server and routing
│   ├── bootstrap.rs        # Startup: check deps
│   ├── hybrid.rs           # Hybrid search (Sourcegraph + Web)
│   ├── ring_log.rs         # Circular buffer logging
│   └── web/
│       ├── mod.rs          # Web search module
│       ├── firecrawl.rs    # Firecrawl API client
│       └── sourcegraph.rs  # Sourcegraph API client
├── tests/
├── Cargo.toml
├── config.jsonc            # Configuration
├── FIRECRAWL_API.md        # Firecrawl API documentation
├── firecrawl/              # Firecrawl repo (submodule)
│   └── apps/api/
│       └── start_local.bat # Local startup script
├── install_firecrawl_service.bat  # Windows service installer
└── run_service.bat         # Start universal-search-service
```

### Running Tests

```bash
cargo test -p universal-search-service
```

---

## Credentials

API keys are configured in `config.jsonc`:

- **Anthropic API Key**: Set in `agent.anthropic_api_key`
- **Anthropic Base URL**: Set in `agent.anthropic_base_url`

These enable LLM-powered agent via the configured proxy.
