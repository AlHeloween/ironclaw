# Project-Agnostic Example Project

This folder is a concrete documentation fixture for ADID skill examples.

It represents a generic project root with common folders that may appear in Python, Delphi, Rust, TypeScript, or mixed-language repositories. Do not treat the files here as a complete application; they exist so published instructions can reference real paths without hard-coding this repository or a user-specific machine path.

## Example Paths

- Installed root: `docs/examples/project-agnostic`
- MCP/RAG config: `docs/examples/project-agnostic/adm.json`
- Tool placeholders: `docs/examples/project-agnostic/tools/README.md`
- Asset pipeline placeholders: `docs/examples/project-agnostic/scripts/`
- Delphi build example: `docs/examples/project-agnostic/delphi/ProjectTool.dpr`
- DUnit example: `docs/examples/project-agnostic/tests/ProjectTests.dpr`

## Substitution Rule

When applying a skill to a real repository, replace `docs/examples/project-agnostic` with that repository's actual root.

The scripts in this fixture intentionally exit with code 2. They document expected file names and must be replaced by real project scripts before use.
