# pub use / pub import re-export

**Status**: open
**Created**: 2026-04-13
**Updated**: 2026-04-13
**ID**: 490
**Depends on**: 234
**Track**: compiler, module-system
**Blocks v1 exit**: no
**Priority**: 50

## Created by audit — 2026-04-13

**Source**: `docs/module-resolution.md` line 213 states "`pub use` or `pub import` makes the imported module's public items re-exported (not yet implemented; tracked in #234)." However, #234's scope and acceptance criteria cover visibility modifiers only (`pub`/`priv`/`pub(crate)`) — re-export was never part of its acceptance. This issue tracks the missing re-export feature separately.

## Summary

The language specification describes `pub use` / `pub import` syntax for re-exporting imported items through the current module's public API. This feature is not implemented; the compiler does not recognise or enforce re-export semantics.

## Acceptance

- [ ] `pub use <module>::<item>` syntax is parsed and resolved
- [ ] Re-exported items are visible to importers of the re-exporting module
- [ ] Non-`pub` uses remain module-private (existing behavior maintained)
- [ ] At least 1 positive fixture (re-export works) and 1 negative fixture (non-pub use not visible)
- [ ] `docs/module-resolution.md` updated to remove "not yet implemented" qualifier

## Primary paths

- `crates/ark-parser/src/`
- `crates/ark-resolve/src/`
- `crates/ark-typecheck/src/`
- `docs/module-resolution.md`

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes
- Re-export fixtures pass

## Close gate

- `pub use` syntax is parsed, resolved, type-checked
- Positive and negative fixtures exist
- docs/module-resolution.md reflects implemented status
