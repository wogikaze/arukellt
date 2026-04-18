# pub use / pub import re-export

**Status**: open
**Created**: 2026-04-13
**Updated**: 2026-04-18
**ID**: 490
**Depends on**: 234
**Track**: compiler, module-system
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
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

- `bash scripts/run/verify-harness.sh --quick` passes *(2026-04-18: PASS — see below)*
- Re-export fixtures pass *(2026-04-18: `modules/pub_use_*` behavioral fixtures PASS in harness; see below)*
- `bash scripts/run/verify-harness.sh --fixtures` full gate *(2026-04-18: FAIL — unrelated failures; see below)*

## Verification evidence — 2026-04-18

Commands (repo root `/home/wogikaze/arukellt`):

```text
$ bash scripts/run/verify-harness.sh --quick
→ PASS (19/19 checks; exit 0)
```

```text
$ bash scripts/run/verify-harness.sh --fixtures
→ FAIL — fixture harness reports 29 failures (exit 1)
  Summary line: PASS: 723 FAIL: 29 SKIP: 31 (scheduled: 783, total manifest: 783)
```

**`pub_use` focus (`tests/fixtures/modules/pub_use_*`):**

- `module-run:modules/pub_use_reexport_visible/main.ark` — not among harness failures; manual check: compile `wasm32-wasi-p1`, `wasmtime run` → stdout `7` (matches `.expected`).
- `module-diag:modules/pub_use_nonpub_hidden/main.ark` — not among harness failures; manual compile fails with `E0501` / `symbol not found in module` as intended.
- `parse-only:module_import/pub_use_basic.ark` — harness skips with `(unknown kind "parse-only")` (expected; manifest bookkeeping only).

**Top unrelated fixture failures (do not mass-fix under this slice):** harness reports multiple buckets, including `FAIL [run] stdlib_io_rw/reader_basic.ark` (empty stdout vs expected), `FAIL [t3-compile] from_trait/from_auto_convert.ark` (stderr warning line), `FAIL [compile-error] component/import_flags_type.ark`, and several `selfhost/*` run fixtures — pre-existing / out of scope for #490.

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

## Progress note — 2026-04-18 (verification evidence)

Re-ran required commands; captured results in **Verification evidence — 2026-04-18** above. `--quick` is now PASS on this snapshot; `--fixtures` still FAIL (29 unrelated failures); `modules/pub_use_*` behavioral fixtures pass harness + spot-check.
