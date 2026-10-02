---
name: patch-tool
description: Repair existing XML update descriptors through adm with ADID backups and per-file ledgers.
kernel-gates: [G7]
---

# patch-tool (adm wrapper for apply_patch)

**Kernel gates:** G7 (repair is a mutation under the adm XML workflow) — see reasoning-kernel.mdc.

Use this only when repairing an existing XML update descriptor through its own XML workflow and you need ADID rotated backups and per-file JSONL ledgers. For ordinary source, documentation, configuration, or test edits, use `apply_patch` directly.

## Command

- Apply a patch file: `tools/adm.exe --patch-tool <patch_file>` (or `python -m adm --patch-tool <patch_file>` in a repo checkout).
- Dry-run (no writes): `tools/adm.exe --dry-run --patch-tool <patch_file>`

## Notes

- This wrapper calls the bundled `apply_patch.exe` and pre-creates rotated backups for any existing target files.
- It emits per-file entries to `<file>.adid.log.jsonl` with `"command": "--patch-tool"` so you can audit what changed.
- Patch files must start with `*** Begin Patch` and end with `*** End Patch`, with file operations like `*** Update File: ...`, `*** Add File: ...`, `*** Delete File: ...`, and `*** Move to: <new_path>` (rename after update).
