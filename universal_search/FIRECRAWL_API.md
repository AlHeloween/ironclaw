# Firecrawl API Documentation

## Overview

Firecrawl is a web scraping and crawling API that converts web content into structured formats (Markdown, HTML, JSON) using LLM-powered extraction. It supports three API versions (v0, v1, v2) with v2 being the current standard.

**Local instance:** `http://localhost:3002`  
**Entry point:** `firecrawl/apps/api/src/index.ts`

## Quick Links

- [Firecrawl Official Docs](https://docs.firecrawl.dev)
- [Firecrawl GitHub](https://github.com/firecrawl/firecrawl)
- [Firecrawl Rust SDK](firecrawl/apps/rust-sdk/)
- [Universal Search Service README](README.md) — Integration with Firecrawl

---

## Configuration

### Environment Variables

The Firecrawl API is configured via environment variables. Key settings for our local setup:

| Variable | Value | Description |
|----------|-------|-------------|
| `PORT` | `3002` | API server port |
| `HOST` | `0.0.0.0` | Bind address |
| `REDIS_URL` | `redis://localhost:6379` | Redis connection |
| `USE_DB_AUTHENTICATION` | `false` | Disable Supabase auth for local use |
| `ANTHROPIC_API_KEY` | *(set in start_local.bat)* | Claude API key for LLM features |
| `ANTHROPIC_BASE_URL` | `https://vanchin.streamlake.ai/api/gateway/coding/ep-4na9u3-1776415346266966985/claude-code-proxy/v1` | Custom Anthropic endpoint |
| `OPENAI_API_KEY` | *(optional)* | OpenAI key for extraction |
| `NUM_WORKERS_PER_QUEUE` | `8` | Worker processes |
| `PLAYWRIGHT_MICROSERVICE_URL` | *(optional)* | Browser automation service |

**Configured in:** `firecrawl/apps/api/start_local.bat`

---

## API Endpoints

### V2 API (Current)

#### 1. Scrape — Single URL

```
POST /v2/scrape
```

**Request:**
```json
{
  "url": "https://example.com",
  "formats": [{"type": "markdown"}],
  "onlyMainContent": true,
  "timeout": 30000
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "markdown": "# Page content...",
    "metadata": {
      "sourceURL": "https://example.com",
      "statusCode": 200,
      "title": "Page Title",
      "description": "...",
      "language": "en"
    }
  }
}
```

**Formats:** `markdown`, `html`, `rawHtml`, `links`, `images`, `screenshot`, `json` (LLM extraction), `summary`, `query` (LLM Q&A)

---

#### 2. Search — Web Search + Scraping

```
POST /v2/search
```

**Request:**
```json
{
  "query": "rust programming",
  "limit": 10,
  "scrapeOptions": {
    "formats": [{"type": "markdown"}],
    "onlyMainContent": true
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "web": [
      {
        "url": "https://example.com",
        "title": "Page Title",
        "description": "Description",
        "markdown": "# Content..."
      }
    ]
  },
  "creditsUsed": 5
}
```

**Note:** Our local Firecrawl instance returns `data` as a **flat array** (not wrapped in `{results: []}`). The universal-search-service handles both formats.

---

#### 3. Crawl — Multi-URL Crawling

```
POST /v2/crawl
```

**Request:**
```json
{
  "url": "https://example.com",
  "limit": 100,
  "scrapeOptions": {"formats": [{"type": "markdown"}]},
  "includePaths": ["/blog/*"],
  "excludePaths": ["/admin/*"]
}
```

**Response:**
```json
{
  "success": true,
  "id": "crawl-uuid",
  "url": "https://api.firecrawl.dev/v2/crawl/crawl-uuid"
}
```

**Status:** `GET /v2/crawl/:jobId`  
**Cancel:** `DELETE /v2/crawl/:jobId`

---

#### 4. Extract — LLM-Powered Data Extraction

```
POST /v2/extract
```

**Request:**
```json
{
  "urls": ["https://example.com"],
  "prompt": "Extract all product names and prices",
  "schema": {
    "type": "object",
    "properties": {
      "products": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "name": {"type": "string"},
            "price": {"type": "number"}
          }
        }
      }
    }
  }
}
```

**Response (async):**
```json
{
  "success": true,
  "id": "extract-uuid"
}
```

**Status:** `GET /v2/extract/:jobId`

**LLM Integration:** Uses `ANTHROPIC_API_KEY` / `ANTHROPIC_BASE_URL` for Claude-based extraction. Falls back to `OPENAI_API_KEY` if Anthropic is not configured.

---

#### 5. Agent — Autonomous AI Agent

```
POST /v2/agent
```

The Agent endpoint provides autonomous web browsing using AI to accomplish complex tasks requiring multiple page interactions.

**Request:**
```json
{
  "urls": ["https://example.com"],
  "prompt": "Find the pricing information and compare all plans",
  "schema": {
    "type": "object",
    "properties": {
      "plans": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "name": {"type": "string"},
            "price": {"type": "number"},
            "features": {"type": "array", "items": {"type": "string"}}
          }
        }
      }
    }
  },
  "maxCredits": 500,
  "model": "spark-1-pro"
}
```

**Models:**
- `spark-1-pro` (default) — Full-featured agent
- `spark-1-mini` — Faster, cheaper

**Response (async):**
```json
{
  "success": true,
  "id": "agent-uuid"
}
```

**Status:** `GET /v2/agent/:jobId`  
**Cancel:** `DELETE /v2/agent/:jobId`

**LLM Integration:** Uses `ANTHROPIC_API_KEY` / `ANTHROPIC_BASE_URL` for the agent's reasoning and decision-making.

---

#### 6. Map — URL Discovery

```
POST /v2/map
```

**Request:**
```json
{
  "url": "https://example.com",
  "search": "pricing",
  "limit": 5000
}
```

**Response:**
```json
{
  "success": true,
  "links": [
    {"url": "https://example.com/pricing", "title": "Pricing"}
  ]
}
```

---

### V1 API (Legacy)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/scrape` | POST | Single URL scrape |
| `/v1/crawl` | POST | Start crawl |
| `/v1/crawl/:jobId` | GET / DELETE | Crawl status / cancel |
| `/v1/search` | POST | Web search |
| `/v1/map` | POST | URL mapping |
| `/v1/extract` | POST | LLM extraction |
| `/v1/extract/:jobId` | GET | Extract status |

---

## Authentication

For local self-hosted use with `USE_DB_AUTHENTICATION=false`, **no API key is required**.

For cloud/production use, include:
```
Authorization: Bearer YOUR_API_KEY
```

---

## Error Responses

```json
{
  "success": false,
  "code": "ERROR_CODE",
  "error": "Human-readable message"
}
```

---

## Starting the Service

### As Windows Service (Recommended)

```batch
universal_search\install_firecrawl_service.bat
```

This installs Firecrawl as a Windows service via NSSM with all environment variables configured.

### Manual Start

```batch
cd universal_search\firecrawl\apps\api
start_local.bat
```

### With Docker (Not Used — Prohibited)

```bash
cd universal_search\firecrawl
docker compose up
```

---

## LLM/Anthropic Configuration

The Firecrawl API uses Anthropic's Claude models for:
- **Extract endpoint** — LLM-powered structured data extraction
- **Agent endpoint** — Autonomous browsing and reasoning
- **JSON format in scrape** — LLM extraction from page content

**Configuration (set in `start_local.bat`):**
```batch
set ANTHROPIC_API_KEY=your_key_here
set ANTHROPIC_BASE_URL=https://your_anthropic_proxy_url/v1
```

When `ANTHROPIC_BASE_URL` is set, all Anthropic API calls are routed through the specified proxy endpoint.

---

## See Also

- [Firecrawl Documentation](https://docs.firecrawl.dev)
- [Firecrawl GitHub](https://github.com/firecrawl/firecrawl)
- `firecrawl/apps/api/src/` — API source code
- `firecrawl/apps/rust-sdk/src/v2/` — Rust SDK with type definitions
- `universal_search/src/web/firecrawl.rs` — Our Firecrawl client integration
- `universal_search/config.jsonc` — Service configuration
