---
Status: "open (reviewed 2026-04-22 — deprecation-marker slice complete; remaining removal work re-scoped to #529)"
Created: 2026-03-31
Updated: 2026-04-22
ID: 285
Depends on: 284
Track: main
Orchestration class: implementation-ready
---
# Legacy lowering path を隔離・撤去する
**Blocks**: (cycle broken — see ADR-028; #508 now depends on #529 instead)
**Track**: corehir
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none
**Priority**: 5

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

**Operational lane**: legacy removal / selfhost transition record. Keep separate from #125/#126 trusted-base compiler default-path correction and from #099 selfhost frontend design.

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

**Blocker history:** [#508 — `issues/open/508-legacy-path-removal-unblocked-by.md`](508-legacy-path-removal-unblocked-by.md)
previously held the fallback-removal work. As of 2026-04-22 / ADR-028, that
work is re-scoped under #529 and no longer makes #285 the active compiler
implementation blocker.

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
- [ ] `lower_corehir_with_fallback` のフォールバック経路を除去 — **re-scoped to #529 via ADR-028**
- [x] `--mir-select legacy` 使用時に deprecation warning を出す（1 リリース後に除去）
- [ ] 全 fixture が legacy なしで pass する — **re-scoped to #529 via ADR-028**

## Resolution path — 2026-04-22 (ADR-028)

[ADR-028](../../docs/adr/ADR-028-corehir-lowering-resolution.md) で
issue \#285 ⇄ \#508 の循環ブロッカーを設計判断で解消した。

- Deprecation marker 部分は完了済みのため、本 issue は近日中にクローズ予定。
- 残アクセプタンス項目 (fallback 除去 / fixture が legacy なしで pass) は
  #529 配下の Rust クレート退役サブイシューに移管する。本 issue の `Blocks:`
  リストから #508 を外し、循環を解消する。
- 詳細は ADR-028 "Sequencing" / "Follow-up sub-issues" セクションを参照。

## Canonical review note — 2026-04-22

ADR-028 is the current source of truth for this tracker. Based on the repo evidence in
`docs/compiler/legacy-path-status.md` and `docs/adr/ADR-028-corehir-lowering-resolution.md`,
the deprecation-marker slice for #285 is complete and is a close candidate. The legacy
fallback itself is still present, but its removal is no longer closure criteria for this
issue; that work was re-scoped under #529. Keep this issue open only as a historical review
record until the queue transition is performed, and do not treat it as the blocker for
CoreHIR implementation work.

## Responsibility split — 2026-04-22

\#285 belongs to the **legacy removal / selfhost transition** lane. It is a
deprecation-marker and historical review record after ADR-028, not the
trusted-base compiler default-path blocker (#125/#126) and not the selfhost
frontend parser/design lane (#099).

## Reviewer checklist — close-candidate only

- [x] ADR-028 explicitly marks the deprecation-marker slice complete and re-scopes the
  remaining fallback-removal work to #529.
- [x] `docs/compiler/legacy-path-status.md` still describes the legacy fallback as present
  and deprecated, which matches a marker-only close candidate.
- [x] `docs/compiler/legacy-path-migration.md` frames the remaining work as staged removal,
  not as already-completed fallback deletion.
- [ ] False-done risk: legacy fallback has been removed from the code path.
- [ ] False-done risk: all fixtures pass without the legacy fallback.
- [ ] False-done risk: #285 is ready to move to `issues/done/` now.

## References

- `crates/ark-mir/src/lower/func.rs`
- `crates/ark-mir/src/lower/facade.rs`
- `docs/compiler/legacy-path-migration.md`
- `docs/compiler/legacy-path-status.md`
- `issues/open/508-legacy-path-removal-unblocked-by.md`