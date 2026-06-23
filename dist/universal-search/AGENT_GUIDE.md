---
title: "Universal Search Agent Usage Guide"
description: "Agent endpoint usage, direct search APIs, configuration, and service management for the Universal Search Service"
category: service
status: production
components:
  - universal_search/src/service.rs
  - universal_search/src/web/firecrawl.rs
  - universal_search/src/web/sourcegraph.rs
  - universal_search/config.jsonc
---

# Universal Search Agent — Usage Guide

## Overview

The Universal Search Service provides three tiers of search capability:

1. **Direct Search APIs** — Web search, URL scraping, code search, hybrid search
2. **Agent Endpoint** — Autonomous AI researcher that uses the search tools iteratively
3. **Service Management** — Health checks, status, configuration

---

## Quick Start

### Start the Service

```batch
# Windows (as service)
nssm start universal-search
nssm start Firecrawl

# Or manually
dist\universal-search\run.bat
```

```bash
# Linux (as service)
sudo systemctl start universal-search
sudo systemctl start firecrawl

# Or manually
cd dist/universal-search && ./universal-search-service run
```

### Verify Health

```bash
curl http://127.0.0.1:3005/health
# => {"status":"healthy","service":"universal-search-service"}
```

---

## Agent Endpoint (AI-Powered Research)

The agent endpoint (`POST /agent`) launches an autonomous research job. The AI uses Claude + Firecrawl search tools to iteratively investigate your query, making multiple tool calls until it reaches a conclusion.

### Start an Agent Job

```bash
curl -X POST http://127.0.0.1:3005/agent \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is the current price of AAPL stock?",
    "max_turns": 5,
    "model": null,
    "system_prompt": null
  }'
```

**Response:**
```json
{
  "success": true,
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "status": "processing"
}
```

### Check Job Status

```bash
curl http://127.0.0.1:3005/agent/a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

**Response (while processing):**
```json
{
  "success": true,
  "status": "processing",
  "current_turn": 2,
  "last_tool": "search"
}
```

**Response (completed):**
```json
{
  "success": true,
  "status": "completed",
  "data": {
    "query": "What is the current price of AAPL stock?",
    "answer": "The current price of AAPL is $173.50...",
    "turns": 3,
    "tool_calls": [
      {
        "turn": 1,
        "tool": "search",
        "input": {"query": "AAPL stock price today"},
        "output": {...}
      },
      ...
    ],
    "model": "claude-sonnet-4-20250514"
  }
}
```

### Cancel a Job

```bash
curl -X DELETE http://127.0.0.1:3005/agent/a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

### Request Parameters

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `query` | string | — | **Required.** Your research question |
| `max_turns` | u32 | 5 | Maximum tool-use iterations |
| `model` | string | config | Override Claude model |
| `system_prompt` | string | config | Override system prompt |

### Available Tools

The agent has access to these tools:

| Tool | Description |
|------|-------------|
| `search` | Web search via Firecrawl — returns scraped results with markdown |
| `scrape` | Extract content from a specific URL — returns markdown |
| `crawl` | Start crawling from a seed URL (returns status only) |
| `map` | Discover all URLs on a domain (not yet implemented) |

### System Prompt

The default system prompt instructs the agent to:
- Use the `search` tool for internet research
- Use `scrape` to read full page content
- Think step-by-step and verify information
- Provide a final answer when confident

Override with `system_prompt` to customize behavior.

---

## Direct Search APIs

Use these for programmatic access without the AI agent loop.

### Web Search (Firecrawl)

```bash
curl -X POST http://127.0.0.1:3005/web/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "rust async programming",
    "count": 5,
    "scrape_formats": "markdown",
    "only_main_content": true
  }'
```

### URL Scraping (Firecrawl)

```bash
curl -X POST http://127.0.0.1:3005/web/context \
  -H "Content-Type: application/json" \
  -d '{
    "query": "https://example.com/article",
    "url": "https://example.com/article",
    "scrape_formats": "markdown",
    "only_main_content": true
  }'
```

### Code Search (Sourcegraph)

```bash
curl -X POST http://127.0.0.1:3005/web/sourcegraph \
  -H "Content-Type: application/json" \
  -d '{
    "query": "lang:rust async fn handler",
    "count": 10
  }'
```

### Hybrid Search (Code + Web)

```bash
curl -X POST http://127.0.0.1:3005/hybrid \
  -H "Content-Type: application/json" \
  -d '{
    "query": "async handler",
    "count": 10
  }'
```

### Direct URL Fetch

```bash
curl -X POST http://127.0.0.1:3005/web/fetch \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com"
  }'
```

---

## Configuration

Edit `universal_search/config.jsonc` and run `build.ps1` to copy it to `dist/universal-search/` (the binary reads it from next to itself). See `README.md` File Layout for details.

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
    "rate_limit_seconds": 10,
    "max_concurrent": 10,
    "anthropic_api_key": "your-key-here",
    "anthropic_base_url": "https://your-proxy/v1"
  },

  "web_search": {
    "firecrawl": {
      "api_url": "http://localhost:3002",
      "api_key": null
    },
    "sourcegraph": {
      "api_url": "https://sourcegraph.com/.api/graphql",
      "access_token": null
    }
  }
}
```

---

## Service Management

### Windows (NSSM)

```batch
# Install
universal_search\install_service.bat
universal_search\install_firecrawl_service.bat

# Control
nssm start universal-search
nssm stop universal-search
nssm status universal-search

# Remove
nssm remove universal-search confirm
```

### Linux (systemd)

```bash
# Install
sudo ./universal_search/install_service.sh
sudo ./universal_search/install_firecrawl_service.sh

# Control
sudo systemctl start universal-search
sudo systemctl stop universal-search
sudo systemctl status universal-search

# Logs
sudo journalctl -u universal-search -f

# Remove
sudo ./universal_search/uninstall_service.sh
```

---

## Troubleshooting

### Agent returns "Rate limited"
Wait `rate_limit_seconds` between requests, or increase the limit in config.

### Agent returns "Max concurrent jobs reached"
Wait for existing jobs to complete, or increase `max_concurrent` in config.

### Firecrawl connection refused
Ensure Firecrawl service is running on port 3002:
```bash
curl http://localhost:3002/health
```

### Agent job stuck in "processing"
Check if the LLM API is reachable. Jobs auto-expire after `job_ttl_seconds` (default: 1 hour).

### Port 3005 already in use
Kill the existing process or change `service.port` in config.

---

## API Reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `GET` | `/status` | Service status |
| `POST` | `/agent` | Start agent job |
| `GET` | `/agent/{id}` | Get job status |
| `DELETE` | `/agent/{id}` | Cancel job |
| `POST` | `/web/search` | Web search |
| `POST` | `/web/context` | URL scraping |
| `POST` | `/web/sourcegraph` | Code search |
| `POST` | `/web/fetch` | Direct HTTPS fetch (no Firecrawl) |
| `POST` | `/hybrid` | Hybrid search |
