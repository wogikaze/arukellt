---
Status: done
Created: 2026-06-12
Updated: 2026-06-15
ID: 637
Track: stdlib
Depends on: 051, 076
Orchestration class: implementation-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 637 — Host capability honesty: fs metadata and read_dir surface

## Summary

docs/stdlib/modules/fs.md documents read_dir, metadata, and is_dir as future or error-returning placeholders. std/manifest.toml and capability docs must honestly reflect implemented vs unavailable surfaces after #051 and #076 land.

## Closure note (2026-06-15)

All acceptance criteria satisfied after #076 landed:

1. **read_dir / metadata / is_dir** — honest semantics in `std/host/fs.ark` and `std/fs/mod.ark`: `is_dir` always `false`; `read_dir`/`metadata` return structured errors documenting the intrinsic gap; `is_file`/`is_readable_file` are read-probes.
2. **std/manifest.toml** — stability labels and availability notes added for `is_readable_file`, `is_file`, `is_dir`, `read_dir`, `metadata`, `fs_error_message`; whole-file I/O keeps `Result<_, String>` to match emitter ABI (FsError reserved for directory/metadata contracts).
3. **docs** — `docs/capability-surface.md`, `docs/stdlib/604-contract-honesty-gap-ledger.md`, and generated `docs/stdlib/modules/fs.md` synced.
4. **Fixture** — `tests/fixtures/stdlib_fs/host_capability_contract.ark` proves probe vs stub boundaries.

## Verification

- `python3 scripts/manager.py verify quick`: 165/165
- `python3 scripts/check/check-docs-consistency.py`: pass (via generate-docs)
- `tests/fixtures/stdlib_fs/host_capability_contract.ark`: registered in manifest.txt
