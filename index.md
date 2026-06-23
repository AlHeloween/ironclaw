# IronClaw Repository Map

Folder-based index of repository contents. Each entry describes the folder's purpose and key entrypoints.

---

## `src/` — Main Rust Source

The IronClaw agent runtime: session/thread/turn management, agent loop, LLM orchestration, tool execution, workspace memory, channel input normalization, and CLI.

| Subfolder | Purpose | Key Entrypoints |
|-----------|---------|-----------------|
| `src/agent/` | Core agent loop, session, scheduler, routines, heartbeat, cost guard | `agent_loop.rs`, `agentic_loop.rs`, `session.rs`, `scheduler.rs` |
| `src/channels/web/` | Browser-facing API/UI — auth, chat, SSE, WebSocket, memory, extensions | `server.rs`, `auth.rs`, `sse.rs`, `handlers/chat.rs` |
| `src/channels/wasm/` | WASM channel runtime for external platforms (Telegram, Slack, etc.) | `runtime.rs`, `host.rs`, `loader.rs` |
| `src/cli/` | CLI subcommands — config, tool, registry, MCP, doctor, search, etc. | `mod.rs`, `config.rs`, `tool.rs`, `service.rs` |
| `src/db/` | Dual-backend persistence (PostgreSQL + libSQL) | `mod.rs`, `postgres.rs`, `libsql_migrations.rs` |
| `src/llm/` | Multi-provider LLM (Anthropic, OpenAI, Gemini, etc.), smart routing, failover | `provider.rs`, `registry.rs`, `smart_routing.rs`, `failover.rs` |
| `src/tools/` | Extensible tool system — built-in, MCP, WASM sandbox, dynamic builder | `tool.rs`, `registry.rs`, `dispatch.rs`, `builtin/mod.rs` |
| `src/tools/mcp/` | Model Context Protocol client (stdio, HTTP, Unix sockets) | `client.rs`, `factory.rs`, `session.rs`, `transport.rs` |
| `src/tools/wasm/` | WASM sandbox (wasmtime) for untrusted tools | `runtime.rs`, `host.rs`, `allowlist.rs`, `limits.rs` |
| `src/workspace/` | Persistent memory — hybrid FTS+vector search, chunking, embeddings | `search.rs`, `embeddings.rs`, `chunker.rs`, `document.rs` |
| `src/extensions/` | Extension lifecycle — discovery, install, activate, remove | `manager.rs`, `discovery.rs`, `registry.rs` |
| `src/setup/` | Onboarding and configuration flow | `mod.rs` |

Root files: `main.rs`, `app.rs`, `search_manager.rs`, `error.rs`

---

## `crates/` — Extracted Shared Crates

Reusable library crates, compiled independently of the main binary.

| Crate | Purpose | Key Entrypoints |
|-------|---------|-----------------|
| `ironclaw_common/` | Shared types and utilities across all crates | `Cargo.toml` |
| `ironclaw_safety/` | Prompt injection detection, content validation, credential leak detection | `lib.rs`, `validator.rs`, `sanitizer.rs`, `policy.rs` |
| `ironclaw_skills/` | SKILL.md parsing, gating, scoring, budget fitting, registry | `lib.rs`, `parser.rs`, `selector.rs`, `registry.rs` |
| `ironclaw_engine/` | Engine v2 types/traits — conversations, threads, capabilities, missions | `traits/mod.rs`, `types/mod.rs`, `capability/mod.rs` |
| `ironclaw_gateway/` | Web gateway frontend assets — HTML/CSS/JS bundle, widgets, themes | `bundle.rs`, `layout.rs`, `widget.rs` |
| `ironclaw_tui/` | Terminal UI (Ratatui) — conversation view, tool panel, approval dialogs | `app.rs`, `render.rs`, `widgets/mod.rs` |

---

## `universal_search/` — Universal Search Service [EXTRACTED]

Extracted to standalone repo: `https://github.com/near/universal-search` (v0.2.0). See `D:/zPython/universal-search/` for the current working copy.
Web search backend: crw-server (Rust) → SearXNG (Python) on port 3434 — full Google/Bing/Wikipedia search. Websurfx (Rust, port 3008) for user-facing web UI.

---

## `docs/` — Project Documentation

| Subfolder | Purpose |
|-----------|---------|
| `docs/capabilities/` | Feature docs — LLM providers, memory, skills, sandboxed tools |
| `docs/channels/` | Channel guides — overview, Telegram, Discord, custom channels |
| `docs/extensions/` | Extension/tool docs — web search, GitHub, MCP, Google Workspace |
| `docs/plans/` | Development plans — E2E infra, engine v2, QA, crate extraction |
| `docs/internal/` | Internal specs — user management API, engine architecture, routing |
| `docs/drafts/` | Draft documents and work-in-progress specs |
| `docs/zh/` | Chinese translations of docs |
| `docs/ADID_Framework_15_3.md` | ADID framework specification |

---

## `tests/` — Test Infrastructure

| Subfolder | Purpose |
|-----------|---------|
| `tests/support/` | Test harness — assertions, cleanup, gateway harness, instrumented LLM, mock servers |
| `tests/e2e/` | Python/Playwright E2E scenarios |
| `tests/fixtures/` | LLM trace recordings, test pages, sample PDFs |

---

## Infrastructure & Config

| Folder/File | Purpose |
|-------------|---------|
| `migrations/` | Database schema migrations (PostgreSQL + libSQL), V1–V23 |
| `profiles/` | Deployment profiles — local, local-sandbox, server, multi-tenant |
| `.github/` | CI/CD workflows, PR template, labeler, Dependabot |
| `wit/` | WIT (WebAssembly Interface Type) definitions for tools and channels |
| `wix/` | Windows Installer (MSI) WiX definition |
| `Cargo.toml` | Root workspace manifest |
| `build.ps1` | Windows build script |
| `AGENTS.md` | Agent quick-start contract |
| `CLAUDE.md` | Claude-specific agent instructions |
| `CHANGELOG.md` | Release changelog |

---

> **Last updated:** 2026-05-19
