# Legacy lowering path を隔離・撤去する

**Status**: open (partially complete — blocked by CoreHIR lowerer stub)
**Created**: 2026-03-31
**Updated**: 2026-04-18
**ID**: 285
**Depends on**: 284
**Blocks**: 508
**Track**: corehir
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none
**Priority**: 5

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.


## Reopened by audit — 2026-04-13

**Reason**: Legacy fallback still active.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Progress update — 2026-04-18

Read-only audit against `docs/compiler/legacy-path-status.md` and
`issues/open/508-legacy-path-removal-unblocked-by.md`: narrative still matches the
pipeline — every compilation still depends on the legacy `lower_to_mir` body because
`lower_hir_to_mir` returns an empty `MirModule` and `lower_corehir_with_fallback` always
falls through to `lower_corehir_via_legacy` → `lower_hir_fallback`.

**What remains for fallback removal (all gated on #508):**

- Implement `lower_hir_to_mir` so it emits real `MirFunction` entries for the fixture
  suite (unblocks empty-module / `ensure_runtime_entry` failures).
- Delete the fallback arm in `lower_corehir_with_fallback` and route compilation only
  through CoreHIR once the above holds.
- Re-run acceptance: all fixtures pass with no legacy path; then remove deprecated
  legacy entrypoints per `docs/compiler/legacy-path-migration.md`.

**Blocker:** [#508 — `issues/open/508-legacy-path-removal-unblocked-by.md`](508-legacy-path-removal-unblocked-by.md).

Orchestration note: #508 **depends on** this issue for deprecation/markers; it **blocks**
the remaining unchecked items here (fallback removal, legacy-less fixtures).

## Progress update — 2026-04-15

Deprecation marking work complete. Full removal blocked by CoreHIR stub.
See `issues/open/508-legacy-path-removal-unblocked-by.md`.

Documentation slice for deprecation/migration guidance is complete. Issue remains open
only for the fallback-removal acceptance items that are blocked by #508.

**Completed:**
- `lower_to_mir()` already had `#[deprecated]` ✓
- `lower_legacy_only`, `lower_prefer_legacy`, `lower_any_to_mir`, `lower_corehir_via_legacy`
  all marked `#[deprecated]` ✓
- `MirSelection::Legacy` and `OptimizedLegacy` marked `#[deprecated]` ✓
- `--mir-select legacy` CLI emits deprecation warning ✓
- test command default changed from `OptimizedLegacy` to `OptimizedCoreHir` ✓
- `ARK_USE_COREHIR` env var no longer needed, ignored ✓
- `docs/compiler/legacy-path-status.md` created documenting pipeline state ✓
- `docs/compiler/legacy-path-migration.md` created with deprecated surface, migration
  examples, and warning/removal strategy ✓
- `docs/compiler/pipeline.md` updated with legacy fallback section ✓
- `docs/compiler/ir-spec.md` corrected to describe CoreHIR as the default route and
  legacy as deprecated ✓
- `issues/open/508-legacy-path-removal-unblocked-by.md` created ✓
- `lower_corehir_with_fallback` annotated with blocker comment (`crates/ark-mir/src/lower/facade.rs`, pointer to #508) ✓

**Blocked:**
The `lower_to_mir` function body (legacy AST lowering) cannot be removed because
`lower_hir_to_mir` is still a stub returning empty MIR. Removing it would break
all fixtures (>> 10). See issue #508.

## Summary

CoreHIR がデフォルトになった後、legacy path (`lower_to_mir` in `func.rs`) を deprecated にマークし、fallback 経路を除去する。二重メンテナンスを終わらせる。

## Current state

- `crates/ark-mir/src/lower/func.rs`: `lower_to_mir` — legacy lowering のメイン実装（deprecated）
- `crates/ark-mir/src/lower/facade.rs`: `lower_hir_to_mir`（stub / 空 MIR）、`lower_corehir_with_fallback`
  （空 MIR のとき legacy にフォールバック）— 実装位置はここに集約
- CoreHIR path と legacy path の両方が常にメンテナンス対象（実 lowering は legacy 側のみ）

## Acceptance

- [x] `lower_to_mir()` に `#[deprecated]` マークを付与
- [ ] `lower_corehir_with_fallback` のフォールバック経路を除去 — **blocked by #508**
- [x] `--mir-select legacy` 使用時に deprecation warning を出す（1 リリース後に除去）
- [ ] 全 fixture が legacy なしで pass する — **blocked by #508**

## References

- `crates/ark-mir/src/lower/func.rs`
- `crates/ark-mir/src/lower/facade.rs`
- `docs/compiler/legacy-path-migration.md`
- `docs/compiler/legacy-path-status.md`
- `issues/open/508-legacy-path-removal-unblocked-by.md`
