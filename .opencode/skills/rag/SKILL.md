---
name: rag
description: Index/query local repositories using adm RAG (adm.json + sqlite) with BGE embedder, dual-quaternion ranking, fd file discovery, and MCP HTTP daemon.
---

# rag (adm RAG)

This skill covers `adm --query ...`, `adm --rag ...`, `adm-rag --init`, `adm-rag --mcp-http`, and `adm-rag --rag-status`.

## Quick Start (first-time users)

```bash
# 1. Install deps, then check environment
pip install torch sentence-transformers
adm-rag --init

# 2. Index the project (fd respects .gitignore, SHA-256 incremental)
adm --rag index my_project .

# 3. Start the model daemon (optional, for sub-second queries)
adm-rag --mcp-http 127.0.0.1 7990 &

# 4. Query instantly (auto-forwards to MCP server if running)
adm --query my_project "how does the DQ signature work?"
```

## Requirements

- `adm.json` must exist in the launch folder (auto-created with defaults if missing).
- **Python 3.13** with `torch` and `sentence-transformers` installed:
  ```bash
  pip install torch --index-url https://download.pytorch.org/whl/cu124
  pip install sentence-transformers
  ```
- `adm-rag --init` checks whether deps are present and advises if missing (detection only, never installs).
- `adm-rag --rag-status` prints the full environment status.
- Default embedder: `BAAI/bge-base-en-v1.5` (768D) via `sentence_transformers`.
- **`fd`** is bundled in `tools/` and used for gitignore-aware file discovery.
- If running from the frozen `adm-rag.exe` without torch, the binary auto-delegates to the system `adm` (pip-installed) via `ADID_RAG_DELEGATE`.

## Commands

| Command | Purpose |
|---|---|
| `adm-rag --init` | Check environment, advise on missing deps |
| `adm-rag --rag-status` | Show full environment status |
| `adm --rag index <name> [roots]` | Create/update index (fd + SHA-256 incremental) |
| `adm --rag status <name>` | Show index docs/chunks count |
| `adm --rag docs <name> [limit]` | List recently indexed documents |
| `adm --rag delete <name>` | Remove index |
| `adm --rag list` | List all indexes |
| `adm --query <name> "<text>"` | Semantic search (auto-forwarded to MCP) |
| `adm --mcp-http [host] [port]` | Start model daemon (one per machine, shared) |

Both `adm` and `adm-rag` accept the same commands. The `adm` binary forwards RAG commands to `tools/adm-rag.exe`.

## File Discovery and Exclusion

- **`fd`** (bundled in `tools/fd.exe`) walks the file tree, respecting `.gitignore` natively.
- Extensions from `include_globs` are passed to `fd --extension` for efficient filtering.
- `exclude_globs` (e.g. `**/dist/**`, `**/build/**`) provide additional application-level exclusion.
- `exclude_patterns` and `add_patterns` allow per-file overrides.
- Fallback: `os.walk` when `fd` is unavailable.

## Forwarding Architecture

```
adm --rag index .
  │
  ├─ frozen (PyInstaller exe)?
  │   YES → _find_rag_helper → tools/adm-rag.exe (sibling, tools/, or PATH)
  │   NO  → handle internally (pip-installed mode, torch available)
  │
adm-rag.exe
  ├─ frozen without torch?
  │   YES → _delegate_to_system_python → find adm on PATH → subprocess
  │         (ADID_RAG_DELEGATE=1 prevents re-forwarding loop)
  │   NO  → normal operation
  │
  └─ Pro tip: `adm-rag --query` auto-detects MCP HTTP on 127.0.0.1:7990
     and forwards instantly for sub-second queries.
```

## MCP HTTP Daemon (shared model)

One MCP server serves all projects on the machine:

```bash
# Terminal 1: start once
adm-rag --mcp-http 127.0.0.1 7990
# → loads BGE model once, stays in memory

# Terminal 2+: instant queries (0.05s)
adm --query projA "search..."
```

Each tool call carries `config_path` — the server reads the correct `adm.json` per project.

## What gets indexed

- Files matching `include_globs` (discovered by `fd` respecting `.gitignore`)
- Force-included paths from `add_patterns`
- Excluded by `exclude_globs`, `exclude_patterns`, and `.gitignore` (via `fd`)
- Structured ADID history docs (`adid://update/...`, `adid://trace/...`)

Code chunks carry tree-sitter structural tags: `symbol_kind:*`, `symbol_name:*`.

## Embedding + retrieval model

- Default: `sentence_transformers` + `BAAI/bge-base-en-v1.5` (768D), batch size 32, normalize on.
- Hybrid RRF: full-vector cosine + dual-quaternion structural signature + SQLite FTS5.
- SE(3) projection head (optional, trainable via `se3_trainer.py`).
- Incremental indexing: SHA-256 content hash per file; unchanged files skipped, changed files atomically replaced.
- Key tunables: `rag.vector_top_k`, `rag.dq_top_k`, `rag.weight_vector`, `rag.weight_dq`, `rag.rrf_k`, `rag.dq_*`.

## Defaults

- Index DB: `.adid_rag/data/<name>.sqlite3`
- Gitignored folders: `.adid_rag/`, `.rag_env/`
- If `adm-rag --rag index` is run without explicit `index_name`, the current directory name is used.
- If `adm-rag --rag delete` is run without explicit `index_name`, the current directory name is used.

## Architecture

```
┌────────────┐     HTTP:7990     ┌──────────────────┐
│  adm-rag   │ ───tools/call───→ │  MCP HTTP server  │
│  --query   │ ←─  JSON-RPC ─── │  (model loaded)   │
└────────────┘                   │  config_path per  │
                                 │  call → multi-DB  │
┌────────────┐                   └──────────────────┘
│  Project A  │── .rag_env/ + adm.json + .adid_rag/data/A.sqlite3
│  Project B  │── .rag_env/ + adm.json + .adid_rag/data/B.sqlite3
└────────────┘
```

## Common smoke queries

- `adm --query <name> "configuration file location"`
- `adm --query <name> "how does dual quaternion signature work"`
- `adm --query <name> "symbol_name:dual_quaternion_signature_8d"`
