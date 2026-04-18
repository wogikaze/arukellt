# pub use / pub import re-export

**Status**: open
**Created**: 2026-04-13
**Updated**: 2026-04-18
**ID**: 490
**Depends on**: 234
**Track**: compiler, module-system
**Blocks v1 exit**: no
**Priority**: 50

## Created by audit — 2026-04-13

**Source**: `docs/module-resolution.md` line 213 states "`pub use` or `pub import` makes the imported module's public items re-exported (not yet implemented; tracked in #234)." However, #234's scope and acceptance criteria cover visibility modifiers only (`pub`/`priv`/`pub(crate)`) — re-export was never part of its acceptance. This issue tracks the missing re-export feature separately.

## Reopened by audit — 2026-04-18

**Reason**: The close gate evidence is self-contradictory. This issue was moved to `done/` even though its own progress notes state that required verification is incomplete and that the issue must remain open until end-to-end verification is green.

**Audit evidence**:

- `issues/open/490-pub-use-pub-import-re-export.md`: "Required verification status for close gate remains incomplete due pre-existing repo drift"
- `issues/open/490-pub-use-pub-import-re-export.md`: "Issue remains open until full required verification is green and acceptance can be checked end-to-end."

**Violated acceptance / close gate**:

- Required verification does not have repo-backed pass evidence.
- Close gate cannot be cited while the issue text itself records unresolved verification drift.

## Summary

The language specification describes `pub use` / `pub import` syntax for re-exporting imported items through the current module's public API. This feature is not implemented; the compiler does not recognise or enforce re-export semantics.

## Acceptance

- [x] `pub use <module>::<item>` syntax is parsed and resolved
- [x] Re-exported items are visible to importers of the re-exporting module
- [x] Non-`pub` uses remain module-private (existing behavior maintained)
- [x] At least 1 positive fixture (re-export works) and 1 negative fixture (non-pub use not visible)
- [x] `docs/module-resolution.md` updated to remove "not yet implemented" qualifier

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

## Progress note — 2026-04-14 (parser slice)

Parser-level acceptance slice landed in commit `87dcbfb`:

- Added parser support for `pub use <module>::<item>` form in `crates/ark-parser/src/parser/decl.rs` and related AST/format plumbing.
- Added parser regression proof in `tests/fixtures/module_import/pub_use_basic.ark` and fixture manifest updates.

Remaining for this issue (not yet done):

- Resolver/typecheck semantics for re-export visibility.
- Positive/negative behavioral fixtures for importer visibility semantics.
- Docs status cleanup after full implementation.

## Progress note — 2026-04-14 (resolver/typecheck slice)

Resolver/typecheck acceptance slice landed in commit `cf701a9`:

- Wired re-export visibility flow in `crates/ark-resolve/src/load.rs`, `crates/ark-resolve/src/analyze.rs`, and `crates/ark-resolve/src/resolve.rs`.
- Updated typecheck integration in `crates/ark-typecheck/src/checker/mod.rs` and `crates/ark-typecheck/src/checker/check_expr.rs`.
- Added positive fixture set `tests/fixtures/modules/pub_use_reexport_visible/*`.
- Added negative fixture set `tests/fixtures/modules/pub_use_nonpub_hidden/*`.

Observed slice-level behavior evidence:

- Positive fixture run prints expected value (`7`).
- Negative fixture check emits `E0501` for hidden non-`pub` export path.

Required verification status for close gate remains incomplete due pre-existing repo drift:

- `bash scripts/run/verify-harness.sh --quick` failed on generated docs drift.
- `bash scripts/run/verify-harness.sh --fixtures` failed on unrelated pre-existing fixture failure (`from_trait/from_auto_convert.ark`).

Issue remains open until full required verification is green and acceptance can be checked end-to-end.
