---
Status: open
Created: 2026-07-17
Updated: 2026-07-17
ID: 828
Track: code-quality
Depends on: none
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: formatter ImportBlockSpan packed-i64 workaround debt
---

# 828 — Restore ImportBlockSpan record return (delete packed i64)

## Summary

`fmt` / `lsp` organize-imports temporarily packs import-block line bounds into a
single `i64` (`first * 1_000_000 + last`) because selfhost record field returns
from `ImportBlockSpan` were unreliable and caused `use` duplication on format.

This is semantic debt tracked by `docs/data/semantic-debt-allowlist.toml` and
forbidden for new code by `.cursor/rules/no-semantic-debt-workarounds.mdc`.

## Exact failure scope

- `src/compiler/fmt/sort_imports.ark` — pack/unpack via `i32_to_i64(1000000)`
- `src/compiler/fmt/prune_imports.ark` — same
- `src/compiler/lsp/organize_imports.ark` — unpack helpers
- `ImportBlockSpan` helper file was removed as an unused `pub` boundary violation;
  reintroduce a non-`pub` (or `mod.ark`-exported) record when the return path works

## Acceptance

1. Import block bounds use a real record (or two explicit returns) with no
   `* 1000000` / `% 1000000` packing.
2. `arukellt fmt` on selfhost entry files does not re-introduce duplicate `use`.
3. Allowlist entries for rule `i32-pair-pack-million` /
   `i32-pair-unpack-million` are removed.
4. `python3 scripts/check/check-semantic-debt.py` passes with an empty match set
   for those rules.

## Verification

```bash
python3 scripts/check/check-semantic-debt.py
python3 scripts/manager.py quality quick
# plus a focused fmt smoke on a fixture that previously duplicated use lines
```

## Removal condition

Close when pack/unpack helpers are gone and allowlist entries for #828 are
deleted.
