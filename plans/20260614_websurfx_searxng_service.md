# Plan: Websurfx as NSSM Service using SearXNG JSON API

**Date:** 2026-06-14
**Status:** draft
**Owner:** universal-search team

---

## Abstract

Websurfx on port 3008 currently scrapes DuckDuckGo/Searx HTML — which fails because DDG anti-bot blocks HTML scraping. Instead, add a new **SearXNG engine** to websurfx that uses SearXNG's native JSON API (no HTML scraping). This makes websurfx a reliable Rust meta-search proxy that delegates upstream search to SearXNG (public instance or self-hosted).

Register websurfx as an NSSM Windows service on port 3008 alongside the existing services.

Universal-search already points to websurfx via `search_backend: "websurfx"` — no changes needed there.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Universal Search Service              │
│                        :3005                            │
│  POST /web/search → execute_websurfx_search()           │
│       │                                                 │
│       │ GET http://localhost:3008/search?format=json&q=X│
│       ▼                                                 │
│  ┌──────────────────────────────────────────────────┐   │
│  │              Websurfx (Rust) :3008               │   │
│  │  NSSM service, auto-start, portable config       │   │
│  │                                                  │   │
│  │  Engines:                                        │   │
│  │   ├─ DuckDuckGo (HTML scrape, unreliable)        │   │
│  │   ├─ Searx    (HTML scrape, unreliable)          │   │
│  │   └─ SearXNG  (JSON API)    ← NEW — primary      │   │
│  │        │                                         │   │
│  │        │ GET {SEARXNG_URL}/search?format=json    │   │
│  │        ▼                                         │   │
│  │   SearXNG (public instance or local)             │   │
│  │        │                                         │   │
│  │        ▼                                         │   │
│  │   Google / Bing / DuckDuckGo / Wikipedia         │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

**No crw-server in the search path.** crw-server stays on :3000 for scraping only (`/v1/scrape`).

---

## Math Formalization

**SearXNG JSON API contract:**
```
Request:  GET {base}/search?format=json&q={query}&pageno={page}&language=en
Response: {
  query: string,
  number_of_results: integer,
  results: [{url, title, content, engine, score, ...}],
  ...
}
```

**Mapping to Websurfx SearchResult:**
```
WebsurfxSearchResult {
  title       ← SearXNG result.title
  url         ← SearXNG result.url
  description ← SearXNG result.content   // note: "content" → "description"
  engine      ← ["searxng"]
}
```

**Mapping to universal-search WebSearchResponse (existing, unchanged):**
```
WebSearchResult {
  title       ← WebsurfxSearchResult.title
  url         ← WebsurfxSearchResult.url
  description ← WebsurfxSearchResult.description
  markdown    ← None
  site_name   ← extract_hostname(url)
}
```

---

## Structural Diagram

```
Files changed/created:
├── websurfx-src/
│   ├── src/engines/searxng.rs          [NEW]  SearXNG JSON engine (~70 lines)
│   ├── src/engines/mod.rs              [EDIT] +pub mod searxng;
│   ├── src/models/engine_models.rs     [EDIT] +match arm for "searxng"
│   └── websurfx/config.lua            [EDIT] +SearXNG = true
│
├── universal-search/
│   └── (no changes — already uses websurfx backend)
│
├── NSSM service registration:
│   └── websurfx service → port 3008, AppDirectory = target/external
```

---

## Implementation

### Step 1: New SearXNG Engine (`src/engines/searxng.rs`)

```rust
//! SearXNG engine — uses SearXNG's native JSON API (no HTML scraping).
//! Configure SEARXNG_URL env var, or defaults to a public instance.

use std::collections::HashMap;
use crate::models::aggregation_models::SearchResult;
use crate::models::engine_models::{EngineError, SearchEngine};
use error_stack::{Report, Result, ResultExt};
use serde::Deserialize;

pub struct Searxng;

#[derive(Debug, Deserialize)]
struct SearxngResponse {
    results: Vec<SearxngResult>,
}

#[derive(Debug, Deserialize)]
struct SearxngResult {
    url: String,
    title: String,
    content: String,
}

#[async_trait::async_trait]
impl SearchEngine for Searxng {
    async fn results(
        &self, query: &str, page: u32, _user_agent: &str,
        _request_timeout: u8, _safe_search: u8,
    ) -> Result<HashMap<String, SearchResult>, EngineError> {
        let base_url = std::env::var("SEARXNG_URL")
            .unwrap_or_else(|_| "https://searx.work".to_string());
        
        let url = format!("{base_url}/search?format=json&q={query}&pageno={page}");
        
        let resp: SearxngResponse = reqwest::get(&url)
            .await
            .change_context(EngineError::RequestError)?
            .json()
            .await
            .change_context(EngineError::RequestError)?;
        
        let map: HashMap<String, SearchResult> = resp.results.into_iter()
            .map(|r| {
                (r.url.clone(), SearchResult::new(
                    &r.title, &r.url, &r.content, &["searxng"]
                ))
            })
            .collect();
        
        if map.is_empty() {
            return Err(Report::new(EngineError::EmptyResultSet));
        }
        Ok(map)
    }
}
```

### Step 2: Register Engine

**`src/engines/mod.rs`** — add `pub mod searxng;`

**`src/models/engine_models.rs`** — add match arm:
```rust
"searxng" => ("searxng", Box::new(crate::engines::searxng::Searxng)),
```

### Step 3: Enable in Config

**`websurfx/config.lua`:**
```lua
upstream_search_engines = {
    DuckDuckGo = false,
    Searx = false,
    SearXNG = true,
}
```

### Step 4: Build

```powershell
cargo build --release
Copy-Item target/release/websurfx.exe target/external/websurfx.exe
```

### Step 5: Register as NSSM Service

```powershell
nssm install websurfx D:\zPython\universal-search\target\external\websurfx.exe
nssm set websurfx AppDirectory D:\zPython\universal-search\target\external
nssm set websurfx AppEnvironmentExtra SEARXNG_URL=https://searx.work
nssm set websurfx AppExit Default Restart
nssm set websurfx DisplayName websurfx
nssm start websurfx
```

### Step 6: Test

```powershell
# Direct websurfx JSON test
curl "http://localhost:3008/search?q=rust+programming&format=json"

# Through universal-search
curl -X POST "http://localhost:3005/web/search" -H "Content-Type: application/json" -d '{"query":"rust programming"}'

# Through universalsearch tool
universalsearch(query="current rust version", source="web")
```

---

## Test Cases

### TC1: SearXNG engine returns results
- **Input:** `curl "http://localhost:3008/search?q=rust+language&format=json"`
- **Expected:** `results` array non-empty, `filtered: false`, each result has `title`, `url`, `description`
- **Oracle:** JSON schema validation

### TC2: Empty query handling
- **Input:** `curl "http://localhost:3008/search?q=&format=json"`
- **Expected:** HTTP 302 redirect or empty results, no crash

### TC3: SearXNG unreachable (network down)
- **Input:** Stop internet, send search request
- **Expected:** `engineErrorsInfo` contains `"searxng"` with error type `"RequestError"`, results empty

### TC4: universal-search integration
- **Input:** `curl -X POST localhost:3005/web/search -d '{"query":"rust"}'`
- **Expected:** Valid `WebSearchResponse` with results > 0

### TC5: universal-searcher tool integration
- **Input:** `universalsearch(query="latest rust version", source="web")`
- **Expected:** Returns search results with titles and URLs

---

## Cleanup: Remove crw-server search dependency

After verification, crw-server's `/v1/search` is no longer needed — websurfx handles all web search. crw-server remains for:

| Endpoint | Used for | Status |
|----------|----------|:---:|
| `/v1/scrape` | URL scraping (execute_context) | Keep |
| `/v1/search` | Web search | **Dead path** — now handled by websurfx |

No code changes needed — the dispatch in `firecrawl.rs` already routes `web_search` to websurfx and `context` to Firecrawl/crw.

---

## Dependencies

| Component | Language | Install | Purpose |
|-----------|----------|:---:|---|
| websurfx | Rust | Already built | Search proxy on :3008 |
| SearXNG | N/A | Public instance (searx.work) | Upstream meta-search |
| rsedis | Rust | Already running | Cache backend for websurfx |
| universal-search | Rust | Already built + deployed | Main API on :3005 |
| crw-server | Rust | Already running | Scraping only on :3000 |

**Zero new dependencies.** No Python. No Docker. No Node.js.

---

## Rollback

1. `nssm stop websurfx; nssm remove websurfx confirm`
2. Revert websurfx config to `DuckDuckGo = true`
3. Restart websurfx via cmd_runner (no longer NSSM service)

---

## Done Checklist

- [ ] `src/engines/searxng.rs` created
- [ ] `src/engines/mod.rs` updated
- [ ] `src/models/engine_models.rs` updated
- [ ] `websurfx/config.lua` updated
- [ ] `cargo build --release` passes (0 warnings)
- [ ] websurfx installed as NSSM service (`searxng-service`)
- [ ] `curl localhost:3008/search?q=test&format=json` returns JSON with results > 0
- [ ] `curl localhost:3005/web/search` returns valid results via websurfx
- [ ] `universalsearch(query="test", source="web")` returns results
- [ ] `cargo test` passes (29/29)
- [ ] Plan moved to `plans_completed/`
