# Firecrawl Search Service

Native Rust service providing web search, code search, URL scraping, and hybrid mode for IronClaw.

## Features

- **Web Search** — Full-page content via Firecrawl API (replaces Brave Search)
- **URL Scraping** — Extract clean markdown from any URL for RAG grounding
- **Code Search** — Search public codebases via Sourcegraph GraphQL API
- **Hybrid Mode** — Combine local code search results with Sourcegraph (local first)

## Quick Start

### 1. Build

```bash
cd src/firecrawl-search-service
cargo build --release
```

The binary is `target/release/firecrawl-search-service`.

### 2. Configure

Create `~/.ironclaw/firecrawl-search.jsonc`:

```jsonc
{
  "service": {
    "port": 3005,
    "bind_address": "127.0.0.1"
  },
  "firecrawl": {
    "api_url": "http://localhost:3002"
  },
  "sourcegraph": {
    "access_token": ""
  },
  "local_search": {
    "url": "http://127.0.0.1:3004",
    "enabled": true
  }
}
```

Use the template at `firecrawl-search.jsonc.template` for all available options.

### 3. Run

```bash
firecrawl-search-service
```

Or with CLI overrides:

```bash
firecrawl-search-service --port 3005 --bind 127.0.0.1
```

## API

### `POST /search` — Web Search
```json
{
  "query": "rust async programming",
  "count": 5,
  "scrape_formats": "markdown",
  "only_main_content": true
}
```

### `POST /context` — URL Scraping
```json
{
  "query": "extract content",
  "url": "https://example.com/article",
  "scrape_formats": "markdown",
  "only_main_content": true
}
```

### `POST /sourcegraph` — Code Search
```json
{
  "query": "async fn handle_request",
  "url": "tokio-rs/tokio",
  "count": 10
}
```

### `POST /hybrid` — Combined Search
```json
{
  "query": "UserManager",
  "url": null,
  "count": 10
}
```

Returns results from both local code search (first) and Sourcegraph (second).

### `GET /health` — Health Check
```json
{"status": "healthy", "service": "firecrawl-search-service"}
```

## Architecture

```
┌─────────────────┐     HTTP      ┌──────────────────────┐
│   IronClaw      │──────────────▶│  Firecrawl Search    │
│   (agent)       │               │  Service (:3005)     │
│                 │               │                      │
│  firecrawl_search│◀─────────────│  - Web search        │
│  (WASM wrapper)  │   JSON       │  - Sourcegraph       │
└─────────────────┘               │  - Hybrid mode       │
                                  │  - Context scraping  │
                                  └──────────────────────┘
                                           │
                                           ▼
                                  ┌──────────────────────┐
                                  │  Local Code Search   │
                                  │  Service (:3004)     │
                                  └──────────────────────┘
```

## Configuration Reference

| Property | Default | Description |
|----------|---------|-------------|
| `service.port` | 3005 | HTTP port |
| `service.bind_address` | 127.0.0.1 | Bind address |
| `firecrawl.api_url` | http://localhost:3002 | Firecrawl API URL |
| `firecrawl.api_key` | (env var) | Firecrawl API key (cloud only) |
| `sourcegraph.api_url` | https://sourcegraph.com/.api/graphql | Sourcegraph endpoint |
| `sourcegraph.access_token` | (env var) | Sourcegraph access token |
| `local_search.url` | http://127.0.0.1:3004 | Local code search service URL |
| `local_search.enabled` | true | Enable local search in hybrid mode |

## Environment Variables

The service also respects these environment variables (config file takes precedence):
- `FIRECRAWL_API_URL` — Override Firecrawl API URL
- `FIRECRAWL_API_KEY` — Firecrawl API key for cloud API
- `SOURCEGRAPH_ACCESS_TOKEN` — Sourcegraph access token

## Integration with IronClaw

The Firecrawl WASM tool (`firecrawl_search`) acts as a thin wrapper:
1. Checks if the native service is running on the configured port
2. Forwards requests to the native service if available
3. Falls back to built-in WASM implementations if service is unavailable

This provides the best of both worlds:
- **Performance**: Native Rust service handles heavy HTTP work
- **Reliability**: WASM fallback ensures search always works
- **Security**: WASM sandbox for approval gates and credential handling
