---
name: agent-assets
description: Maintain canonical artefacts and install agent receiver scaffolds (.cursor/.codex/~/.codex/.opencode).
---

# agent-assets

## Canonical sources

- Rules: `artefacts/rules/`
- Skills: `artefacts/skills/`

Agent folders are receivers (safe to delete): `.cursor/`, `.codex/`, `~/.codex/`, `.opencode/`.

## Workflow

1. Edit canonical assets under the asset-source repository's rules and/or skills folders.
2. Regenerate derived artefacts and scaffolds using that repository's real asset build command.

Concrete fixture for expected script names: `artefacts/examples/project-agnostic/scripts/`.

3. Install scaffolds into receivers using the repository's real sync command.

Replace the fixture paths with real project scripts before running commands.

## Targets

- Install only one receiver:
  - `python <real_asset_pipeline>/sync_agent_assets.py --targets opencode`
