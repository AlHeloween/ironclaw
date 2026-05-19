# Documentation Surface Index

Owner-tracked index of documentation files that claim a working state (`production`, `test`, or `execution`). Each entry must include the YAML frontmatter header and a `reproduce:` block.

Separate from `index.md` which is the folder-based repository map.

---

## Production

| Document | Owner | Entrypoints | Last Verified |
|----------|-------|-------------|:---:|
| `universal_search/README.md` | universal_search team | Service README, reproduce block | 2026-05-19 |
| `universal_search/AGENT_GUIDE.md` | universal_search team | Agent endpoint usage, API reference | 2026-05-19 |
| `docs/extensions/web-search.md` | extensions team | Web search extension setup | 2026-03-04 |

## Internal Specs

| Document | Owner | Entrypoints | Last Verified |
|----------|-------|-------------|:---:|
| `docs/internal/engine-v2-architecture.md` | engine team | Engine v2 architecture | 2026-03-20 |
| `docs/internal/USER_MANAGEMENT_API.md` | web gateway team | User management API | 2025-12-01 |
| `docs/internal/smart-routing-spec.md` | LLM team | Smart routing specification | 2025-12-15 |
| `docs/internal/self-improvement.md` | agent team | Self-improvement mission spec | 2026-02-01 |

## Plans (Historical)

| Document | Owner | Status | Date |
|----------|-------|--------|------|
| `docs/plans/2026-03-27-memory-retrieval-ingestion.md` | workspace team | completed | 2026-03-27 |
| `docs/plans/2026-03-25-python-orchestrator.md` | engine team | completed | 2026-03-25 |
| `docs/plans/2026-03-24-missions.md` | agent team | completed | 2026-03-24 |
| `docs/plans/2026-03-23-self-improving-engine.md` | agent team | completed | 2026-03-23 |
| `docs/plans/2026-03-23-engine-v2-security.md` | engine team | completed | 2026-03-23 |
| `docs/plans/2026-03-22-crate-extraction-and-cleanup.md` | core team | completed | 2026-03-22 |
| `docs/plans/2026-03-20-engine-v2-architecture.md` | engine team | completed | 2026-03-20 |
| `docs/plans/2026-02-24-e2e-infrastructure.md` | testing team | completed | 2026-02-24 |
| `docs/plans/2026-02-24-e2e-infrastructure-design.md` | testing team | completed | 2026-02-24 |
| `docs/plans/2026-02-24-automated-qa.md` | testing team | completed | 2026-02-24 |

## Drafts

| Document | Owner | Status |
|----------|-------|--------|
| `docs/drafts/smart-routing-spec.md` | LLM team | draft |
| `docs/drafts/solutions/integration-issues/playwright-screenshot-pipeline.md` | docs team | solved |
| `docs/drafts/solutions/integration-issues/playwright-screenshot-token-auth.md` | docs team | solved |

---

## Reproducibility Checklist

Documents in the **Production** section above must satisfy:

- [x] YAML frontmatter with `title`, `description`, `category`, `status`
- [x] `reproduce:` block with step-by-step verification commands
- [x] Build/run/test commands produce expected output
- [x] No hardcoded secrets in code samples
- [x] Last verified date is current (within 30 days)

---

## Adding a Document

1. Place the document in the appropriate `docs/` subfolder
2. Add YAML frontmatter: `title`, `description`, `category`, `status`
3. For `production`/`test`/`execution` status, add a `reproduce:` block
4. Add an entry to this `DOCINDEX.md` under the correct section
5. Run the reproduce block and update `last_verified`

---

> **Last updated:** 2026-05-19
