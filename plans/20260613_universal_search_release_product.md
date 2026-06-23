---
date: 2026-06-13
title: "Universal Search Service — Standalone Release Product"
status: execution
---

```YAML
master_plan_description: "Extract universal_search from ironclaw monorepo into standalone release product with native binary distribution, simplified external-dependency model, ZIP packaging, and CI/CD."
SV for goal 1 — Extract to standalone repo, 0%
  SV for task 1 — Create directory structure and Cargo.toml, 0%
  SV for task 2 — Copy and adapt all source files, 0%
  SV for task 3 — Create .gitignore, LICENSE, CI workflow, 0%
SV for goal 2 — Refactor bootstrap to health-checks-only, 0%
  SV for task 1 — Rewrite bootstrap.rs (remove auto-clone/pnpm/git/firecrawl-start), 0%
  SV for task 2 — Update config.rs (remove auto_start, repo_path, commit, auto_clone, auto_install_deps), 0%
  SV for task 3 — Update main.rs (remove Bootstrap/InstallFirecrawl commands), 0%
  SV for task 4 — Add graceful 503 in service.rs when deps down, 0%
SV for goal 3 — Build system and packaging, 0%
  SV for task 1 — Update build.ps1 with ZIP packaging and version from Cargo.toml, 0%
  SV for task 2 — Create build.sh for Linux, 0%
  SV for task 3 — Verify build compiles, 0%
SV for goal 4 — Configuration, install, docs, 0%
  SV for task 1 — Update config.jsonc.template for standalone use, 0%
  SV for task 2 — Create README.md, CHANGELOG.md, index.md, DOCINDEX.md, 0%
  SV for task 3 — Create install helper scripts for Firecrawl + Chromium, 0%
  SV for task 4 — Update AGENT_GUIDE.md, 0%
SV for goal 5 — Ironclaw cleanup, 0%
  SV for task 1 — Remove universal_search/ from ironclaw monorepo, 0%
  SV for task 2 — Update ironclaw workspace members, 0%
```

## Design Decisions

| Decision | Choice |
|----------|--------|
| Distribution | Native binary + install scripts (ZIP archive) |
| Firecrawl | External service (NSSM/systemd) — assumed running |
| Agent endpoint | Included, config-driven via config.jsonc |
| Repository | Extracted to own GitHub repo: `D:/zPython/universal-search/` |
| Version | 0.2.0 (bumped from 0.1.0 in ironclaw) |

## Implementation Details

### Bootstrap refactoring
- Remove: auto_clone_firecrawl, auto_install_deps, pnpm, git clone, harness patching, Firecrawl child process
- Keep: TCP health checks for PostgreSQL (:5432), Redis (:6379), Firecrawl API (:3002), Chromium CDP (:9222)
- New struct: `BootstrapResult { pg_ok, redis_ok, firecrawl_ok, chromium_ok, warnings }`
- Service degrades gracefully — endpoints that need a down dep return 503

### Config field changes
- FirecrawlConfig: remove `auto_start`, `repo_path`, `commit`; keep `api_url`, `api_key`, `source`, `postgres?`, `redis?`
- BootstrapConfig: empty (no more auto-clone/install flags)
- Config: remove `firecrawl_repo_path()`, `resolve_paths()`, `make_paths_relative()`, `make_paths_absolute()`

### CLI commands removed
- `bootstrap` — no longer needed (Firecrawl is external)
- `install-firecrawl` — no longer needed
- `convert-paths` — no repo_path to convert

### Test cases
- Existing unit tests in config.rs, fetch.rs, ring_log.rs — preserve
- integration test in tests/test_integration.py — update paths
- Sourcegraph/firecrawl mock tests — preserve
