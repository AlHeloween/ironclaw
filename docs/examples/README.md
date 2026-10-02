# ADID Examples

Documentation fixtures for published ADID skills and rules: a minimal generic
project-root example (`adm.json`, Delphi build, DUnit tests, tools placeholders).
They exist so skills can reference concrete paths without pointing at this
repository or a developer machine. Fixtures are not runnable projects.

When applying a skill to a real repository, replace `docs/examples` paths with
that repository's actual root.

## Contents

- `adm.json` — example RAG/MCP configuration
- `delphi/` — example Delphi project (`ProjectTool.dpr`)
- `tests/` — example DUnit test project (`ProjectTests.dpr`, `TestCore.pas`, `TestServices.pas`)
- `tools/` — tool placeholders README
