# ADID RAG — Standalone Usage Guide

ADID RAG provides local semantic search over your codebase using
`fd` for gitignore-aware file discovery and the BGE embedding model
for vector search with dual-quaternion structural reranking.

## Installation

**Option 1: One-click installer (recommended)**

Double-click `install_rag.cmd` (Windows) or run `./install_rag.sh` (Linux).
This installs Python 3.13, PyTorch, sentence-transformers, and the ADID package.

**Option 2: Manual pip install**

```bash
pip install dist/adm-5.0.6-py3-none-any.whl[rag]
```

**Option 3: From PyPI**

```bash
pip install adm[rag]
```
```

Prints environment status — detection only, never downloads or installs.
If deps are missing, it shows the pip commands to run.

## Per-Project Setup

```bash
cd my-project
adm --rag index my_project .   # fd respects .gitignore, SHA-256 incremental
```

The index lands at `.adid_rag/data/my_project.sqlite3`.

## Querying

```bash
# Sub-second queries via MCP server:
adm-rag --mcp-http 127.0.0.1 7990 &   # start once, model stays loaded
adm --query my_project "how does the DQ signature work?"

# Standalone query (model loaded per invocation, ~10s):
adm --query my_project "how does the DQ signature work?"
```

The `--query` command auto-detects the MCP server on `127.0.0.1:7990`
and forwards instantly when available.

## MCP Server (Shared Model)

One MCP server serves all projects on the machine:

```bash
# Terminal 1: start server
adm-rag --mcp-http 127.0.0.1 7990

# Terminal 2+: instant queries from any project
cd /path/to/projectA && adm --query projA "search..."
cd /path/to/projectB && adm --query projB "search..."
```

Each tool call carries `config_path` — the server reads the correct
`adm.json` per project.

## Architecture

```
adm --query → [frozen exe?] → tools/adm-rag.exe → [no torch?] → system adm/python
                  │                                            │
            pip-installed mode                          torch available
                  │                                            │
                  └──────────── MCP HTTP :7990 ────────────────┘
```

- `adm.exe` (frozen, 20MB) forwards RAG commands to `tools/adm-rag.exe`
- `adm-rag.exe` (frozen, 21MB) delegates to system `adm` when torch is missing
- System `adm` (pip-installed) has full access to torch for embedding
- One MCP daemon (`--mcp-http`) loads the model once for all projects

## File Discovery and Exclusion

- **`fd`** (bundled in `tools/fd.exe`) walks the file tree, respecting all
  `.gitignore` files (root and nested)
- Extensions from `include_globs` passed via `fd --extension` filters
- Additional application-level exclusions via `exclude_globs`/`exclude_patterns`
- Falls back to `os.walk` when `fd` is unavailable

## Incremental Indexing

Re-indexing is safe and fast:
- Unchanged files: **skipped** (SHA-256 content hash match)
- Changed files: old chunks **replaced** atomically
- New files: indexed normally
- Embedder/config change: **full wipe** and re-index

## Diagnose

```bash
adm-rag --rag-status              # full environment health
adm --rag docs my_project         # recently indexed files
adm --rag status my_project       # index statistics
```

## Updating

```bash
# Re-run the installer (detects and upgrades):
install_rag.cmd         # Windows
./install_rag.sh        # Linux

# Or manual:
pip install --upgrade adm[rag]
```

## Troubleshooting

| Problem | Fix |
|---|---|
| "torch not installed" | `pip install torch sentence-transformers` |
| "CUDA not available" | CPU fallback works; install CUDA torch for speed |
| MCP port in use | Change port: `--mcp-http 127.0.0.1 7991` |
| Query returns nothing | Re-index: `adm --rag index <name> .` |
| "adm-rag.exe not found" | Copy from `tools/` or add to PATH |
