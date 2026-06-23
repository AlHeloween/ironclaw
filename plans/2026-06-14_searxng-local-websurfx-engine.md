# Plan: SearXNG Local + Websurfx SearXNG Engine

**Date:** 2026-06-14
**Status:** pending approval
**Summary:** Run SearXNG locally on port 3434 with rsedis as cache. Add a native SearXNG JSON-API engine to Websurfx. Universal-search calls Websurfx which calls SearXNG which proxies Google/Bing/DDG.

---

## Architecture Target

```
universal-search :3005 /web/search     (NSSM service — stays as-is)
  → websurfx :3008 /search?format=json (NSSM service — stays as-is)
    → SearXNG :3434 /search?format=json (new Python process)
      → Google / Bing / DDG / Wikipedia
        ↓
      rsedis :6379 (cache/limiter)
```

**Port map:**
| Port | Service | Type | Role |
|-----:|---------|------|------|
| 3005 | universal-search | Rust binary (NSSM) | Main API |
| 3008 | websurfx | Rust binary (NSSM) | Meta-search UI + JSON API |
| 3434 | SearXNG | Python 3 | Search engine proxy |
| 6379 | rsedis | Rust binary (NSSM) | Redis-compatible cache |
| 3000 | crw-server | Rust binary (NSSM) | Scraping only (no search) |
| 9222 | chromium-debug | Browser (NSSM) | CDP for Playwright |

---

## Goal 1: Install and Configure SearXNG Locally

**Rationale:** Replace the dead `searx.work` public instance with a local SearXNG. SearXNG needs Python 3 but NO Docker, NO Valkey (rsedis handles cache).

### Task 1.1: Install SearXNG via pip

```bash
python -m pip install searxng
```

Or for isolated install:
```bash
python -m venv .venv/searxng
.venv/searxng/Scripts/pip install searxng
```

### Task 1.2: Create settings.yml

File: `D:\zPython\universal-search\target\external\searxng\settings.yml`

```yaml
use_default_settings: true
search:
  formats:
    - html
    - json
server:
  secret_key: "ironclaw-local-searxng-key-2026"
  bind_address: "127.0.0.1"
  port: 3434
redis:
  url: redis://127.0.0.1:6379/0
```

### Task 1.3: Start SearXNG

```bash
set SEARXNG_SETTINGS_PATH=D:\zPython\universal-search\target\external\searxng\settings.yml
python -m searxng.webapp
```

Or register as NSSM service: `nssm install searxng ...`

### Task 1.4: Verify

```bash
curl "http://127.0.0.1:3434/search?q=test&format=json"
```

Expected: `{"results":[...], "number_of_results": ...}`

---

## Goal 2: Add SearXNG Engine to Websurfx

**Rationale:** Websurfx's existing `Searx` engine scrapes HTML from `searx.work` (dead). Replace with a new `Searxng` engine that uses SearXNG's native JSON API — zero HTML scraping, fully reliable.

### Task 2.1: Create `src/engines/searxng.rs`

New file in websurfx-src. The engine:
- GETs `{searxng_url}/search?format=json&q={query}&pageno={page}&safesearch={safe_search}`
- Deserializes JSON directly (no HTML scraping)
- Maps `content` → `description` (SearXNG uses `content`, Websurfx uses `description`)

```rust
// Pseudocode structure:
pub struct Searxng;

#[async_trait::async_trait]
impl SearchEngine for Searxng {
    async fn results(&self, query, page, user_agent, request_timeout, safe_search) -> Result<HashMap<String, SearchResult>, EngineError> {
        let url = format!("{base}/search?format=json&q={query}&pageno={page}&safesearch={safe_search}");
        let resp = reqwest::get(&url).await?;
        let data: SearxngResponse = resp.json().await?;
        // Map data.results → HashMap<url, SearchResult>
        // Map SearXNG field "content" → Websurfx field "description"
    }
}
```

Files to create/modify:
| File | Action |
|------|--------|
| `src/engines/searxng.rs` | **CREATE** — ~80 lines |
| `src/engines/mod.rs` | + `pub mod searxng;` |
| `src/models/engine_models.rs` | + `"searxng" => ("searxng", Box::new(crate::engines::searxng::Searxng))` |

### Task 2.2: Update websurfx config

File: `websurfx/config.lua`
```lua
upstream_search_engines = {
    DuckDuckGo = false,
    Searx = false,
    Searxng = true,
}
```

Add SearXNG URL setting (in websurfx's config parser or via env var).

### Task 2.3: Rebuild websurfx

```bash
cargo build --release
```
Copy binary to `target/external/websurfx.exe`.

---

## Goal 3: Universal-Search Cleanup

**Rationale:** Remove the unused websurfx dispatch code. The websurfx path was added during experimentation but the correct architecture is websurfx → SearXNG, not universal-search → websurfx directly.

Wait — universal-search already calls websurfx via `search_backend: "websurfx"`. This path WORKS. We should KEEP IT.

**Decision: KEEP the websurfx backend in universal-search.** It's already working, already tested with 29/29 tests passing. The path is:
```
universal-search :3005 → websurfx :3008 → SearXNG :3434 → Google
```

No code changes needed in universal-search. Just ensure config points to websurfx.

### Task 3.1: Verify universal-search config

File: `D:\zPython\universal-search\target\release\config.jsonc`
```json
{
  "web_search": {
    "firecrawl": {
      "search_backend": "websurfx",
      "websurfx_url": "http://localhost:3008"
    }
  }
}
```

Already configured. No change needed.

---

## Goal 4: Service Registration (NSSM)

Convert all processes to NSSM services for auto-start reliability.

| Service Name | Binary | Port | Depends On |
|-------------|--------|-----:|------------|
| rsedis | `target/external/rsedis.exe` | 6379 | — |
| searxng | Python `searxng.webapp` | 3434 | rsedis |
| websurfx | `target/external/websurfx.exe` | 3008 | searxng |
| universal-search | `target/release/universal-search-service.exe` | 3005 | websurfx |

### Currently registered:
- `rsedis` (NSSM) — already running
- `crw-server` (NSSM) — already running on 3000
- `chromium-debug` (NSSM) — already running on 9222
- `universal-search` (NSSM) — already running on 3005

### Need to add:
- `searxng` — NSSM wrapping Python
- `websurfx` — convert from cmd_runner to NSSM

---

## Goal 5: End-to-End Verification

### Task 5.1: Test SearXNG directly
```bash
curl "http://127.0.0.1:3434/search?q=current+events&format=json"
# Expect: real search results from Google/Bing
```

### Task 5.2: Test websurfx → SearXNG
```bash
curl "http://127.0.0.1:3008/search?q=current+events&format=json"
# Expect: websurfx wraps SearXNG results
```

### Task 5.3: Test universal-search → websurfx → SearXNG
```bash
curl -X POST http://127.0.0.1:3005/web/search \
  -H "Content-Type: application/json" \
  -d '{"query":"current events"}'
# Expect: WebSearchResponse with real results
```

### Task 5.4: Run Rust test suite
```bash
cargo test -- --skip ring_log
# Expect: 29/29 pass (as before)
```

---

## Goal 6: Documentation

| Document | Update |
|----------|--------|
| `FEATURE_PARITY.md` | Update search services section — SearXNG local + websurfx |
| `index.md` | Add SearXNG to port map |
| `universal_search/README.md` | Update architecture diagram |
| `plans/2026-06-14_searxng-local-websurfx-engine.md` | Mark complete, move to `plans_completed/` |

---

## Files Changed (Summary)

| File | Action | Lines |
|------|--------|------:|
| `websurfx-src/src/engines/searxng.rs` | CREATE | ~80 |
| `websurfx-src/src/engines/mod.rs` | +1 line | 1 |
| `websurfx-src/src/models/engine_models.rs` | +1 match arm | 1 |
| `websurfx-src/websurfx/config.lua` | Edit engines list | 3 |
| `websurfx/websurfx/config.lua` | Edit engines list | 3 |
| `searxng/settings.yml` | CREATE | ~12 |
| `target/release/config.jsonc` | No change (already correct) | 0 |

**Total:** ~100 lines new, 6 lines modified, 0 deleted.

---

## Dependencies Added

| Dependency | Type | Reason |
|-----------|------|--------|
| `searxng` (pip) | Python package | Search engine proxy |
| None (Rust) | — | No new crates needed |

---

## Risk Assessment

| Risk | Mitigation |
|------|-----------|
| SearXNG fails to start on Windows | Use `python -m searxng.webapp` with explicit settings path; fallback to Docker if needed |
| rsedis incompatible with SearXNG cache | SearXNG cache is optional — disable `redis:` in settings.yml if rsedis fails basic SET/GET |
| SearXNG blocked by Google | SearXNG rotates user agents and supports 200+ engines — not single-point dependent |
| SearXNG Python overhead | Single Python process, ~50MB RAM — acceptable |

---

## Test Cases

| # | Test | Expected |
|---|------|----------|
| 1 | `curl localhost:3434/search?q=test&format=json` | JSON with results array, `number_of_results > 0` |
| 2 | `curl localhost:3008/search?q=test&format=json` | Websurfx JSON wrapping SearXNG results, no HTML |
| 3 | `curl -X POST localhost:3005/web/search -d '{"query":"test"}'` | WebSearchResponse with real results |
| 4 | `cargo test` | 29/29 pass |
| 5 | `universalsearch` tool via opencode | Returns web results, not empty |
| 6 | SearXNG without rsedis | Graceful degradation — search still works, cache disabled |
