# Local Code Search Service

A separate binary service that indexes local codebases and provides full-text + symbol search for IronClaw.

## Features

- **Full-text search** using tantivy (BM25 scoring)
- **Symbol search** for Rust, TypeScript, JavaScript, Python
- **File watcher** for automatic re-indexing
- **Persistent indexes** stored in `~/.ironclaw/code-search-indexes/`
- **HTTP API** on port 3004 (configurable)
- **Hybrid mode** in Firecrawl tool combines local + Sourcegraph search

## Quick Start

### 1. Build

```bash
cd src/local_code_search
cargo build --release
```

The binary is `target/release/local-code-search`.

### 2. Configure

Create `~/.ironclaw/local-code-search.jsonc`:

```jsonc
{
  "service": {
    "port": 3004,
    "bind_address": "127.0.0.1",
    "watch_enabled": true
  },
  "indexes": [
    {
      "name": "ironclaw",
      "path": "/path/to/ironclaw",
      "languages": ["all"],
      "symbols_enabled": true,
      "enabled": true
    }
  ]
}
```

Use the template at `local-code-search.jsonc.template` for all available options.

### 3. Run

```bash
local-code-search --config ~/.ironclaw/local-code-search.jsonc
```

Or with CLI overrides:

```bash
local-code-search --port 3004 --bind 127.0.0.1
```

### 4. Use with Firecrawl

Set the `LOCAL_CODE_SEARCH_URL` env var (default: `http://127.0.0.1:3004`):

```bash
export LOCAL_CODE_SEARCH_URL="http://127.0.0.1:3004"
```

Then use the Firecrawl tool with `mode: "hybrid"`:

```python
result = firecrawl_search(query="async fn", mode="hybrid")
```

This searches both:
1. **Local indexes** (your codebases) — results appear first
2. **Sourcegraph** (public GitHub repos) — results appear second

## API

### `POST /search`
Full-text search across all indexes (or a specific index).

```json
{
  "query": "async fn handle_request",
  "index": "ironclaw",
  "limit": 10
}
```

### `POST /symbol-search`
Search for symbols by name across all indexes.

```json
{
  "query": "CodeIndex",
  "limit": 10
}
```

### `POST /index`
Create and populate a new index.

```json
{
  "name": "my-project",
  "path": "/path/to/my-project"
}
```

### `POST /index/rebuild`
Rebuild an existing index.

```json
{
  "name": "ironclaw",
  "path": "/path/to/ironclaw"
}
```

### `GET /index/stats`
Get stats for all indexes.

### `GET /health`
Health check endpoint.

## Architecture

```
┌─────────────────┐     HTTP      ┌──────────────────────┐
│   IronClaw      │──────────────▶│  Local Code Search   │
│   (agent)       │               │  Service (:3004)     │
│                 │               │                      │
│  firecrawl_search│◀─────────────│  - tantivy indexes   │
│  mode="hybrid"   │   JSON       │  - file watcher      │
└─────────────────┘               │  - symbol extraction │
                                  └──────────────────────┘
                                           │
                                           ▼
                                  ┌──────────────────────┐
                                  │  ~/.ironclaw/        │
                                  │  code-search-indexes/│
                                  │  - ironclaw/         │
                                  │  - my-project/       │
                                  └──────────────────────┘
```

## Symbol Extraction

Regex-based extraction for:
- **Rust**: `fn`, `struct`, `trait`, `enum`, `impl`
- **TypeScript/JavaScript**: `function`, `class`, `interface`, `export`
- **Python**: `def`, `class`

Symbols are stored as JSON in the tantivy index and searchable via `/symbol-search`.

## Configuration Reference

| Property | Default | Description |
|----------|---------|-------------|
| `service.port` | 3004 | HTTP port |
| `service.bind_address` | 127.0.0.1 | Bind address |
| `service.watch_enabled` | true | Enable file watcher |
| `service.watch_debounce_ms` | 500 | Watcher debounce delay |
| `indexes[].languages` | ["all"] | Languages to index |
| `indexes[].symbols_enabled` | true | Enable symbol extraction |
| `indexes[].max_file_size` | 1048576 | Max file size (1MB) |
| `indexes[].exclude` | (common dirs) | Exclude patterns |
