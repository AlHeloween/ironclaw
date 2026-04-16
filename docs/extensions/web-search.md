---
title: "Web Search"
description: "Let your agent search the web using Firecrawl"
---

The Web Search tool allows your agent to search the web and scrape content using a self-hosted [Firecrawl](https://firecrawl.dev) instance. It returns clean markdown from web pages, handles JavaScript-rendered content, and requires no API key for local instances.

The tool supports two modes:
- **search**: Web search with full-page content extraction
- **context**: Scrape a specific URL for RAG grounding

---

## Setup

<Steps>

<Step title="Start a Firecrawl Instance">

### Local (Recommended)

Firecrawl can be self-hosted for free. If you have the IronClaw repository checked out:

```bash
cd externals/firecrawl/apps/api
pnpm install
pnpm run start
```

The API will be available at `http://localhost:3002`.

### Docker

```bash
docker run -d -p 3002:3002 --name firecrawl firecrawl/firecrawl
```

### Cloud API

Use the hosted Firecrawl API at [firecrawl.dev](https://firecrawl.dev). You'll need an API key.

</Step>

<Step title="Install the Web Search Extension">

The extension is included in the default registry. Install it with:

```bash
ironclaw registry install web_search
```

</Step>

<Step title="Configure (Local)">

For local instances, no API key is required. The tool automatically connects to `http://localhost:3002`.

If your Firecrawl instance runs on a different URL, set the environment variable:

```bash
export FIRECRAWL_API_URL=http://your-host:3002
```

</Step>

<Step title="Configure (Cloud)">

If using the cloud API, configure your API key:

```bash
ironclaw tool auth web_search
```

Or set the environment variable:

```bash
export FIRECRAWL_API_KEY=fc-YOUR_API_KEY
export FIRECRAWL_API_URL=https://api.firecrawl.dev
```

</Step>

</Steps>

---

## Usage

### Web Search

```json
{
  "query": "rust programming",
  "mode": "search",
  "count": 5
}
```

### RAG Grounding

```json
{
  "query": "summarize this page",
  "mode": "context",
  "url": "https://docs.rs"
}
```
