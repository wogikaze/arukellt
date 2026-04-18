# `compile()` のデフォルトを CoreHIR パスに移行 (Legacy パス廃止)

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 125
**Depends on**: —
**Track**: pipeline-refactor
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: CoreHIR-default-blocked (issue body: fixtures failing)
**Blocks v4 exit**: yes

**Status note**: BLOCKED — CoreHIR path fails 378/410 fixtures. Legacy path must remain default.

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/125-corehir-as-default-compile-path.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`ark-driver/src/session.rs` の `compile()` は現在 `MirSelection::Legacy` をデフォルトとして使用している。

```rust
// session.rs:410
pub fn compile(&mut self, path: &Path, target: TargetId) -> Result<Vec<u8>, String> {
    self.compile_selected(path, target, MirSelection::Legacy)
}
```

計画されているパイプライン `Check+BuildCoreHIR → LowerToMIR` では CoreHIR が primary path だが、
`compile()` は Legacy を使い続けている。この不整合を解消し、CoreHIR をデフォルトにする。

## 現状

- `run_frontend()` は常に Legacy MIR と CoreHIR MIR の **両方** を生成する (二重 lower)
- `compile()` は `MirSelection::Legacy` でのみ呼ばれる
- `compile_selected(..., MirSelection::CoreHir)` は存在するが、CLI から呼ばれない
- v4 パイプライン仕様は CoreHIR → MIR → BackendPlan の単一経路を前提にしている

## 受け入れ条件

1. `compile()` が `MirSelection::CoreHir`（または最終的に `OptimizedCoreHir`）をデフォルトで使うこと
2. すべての fixture (`cargo test -p arukellt --test harness`) が CoreHIR パスで green
3. `scripts/run/verify-harness.sh` が status 0 で終了
4. Legacy MIR パスを `--mir-select=legacy` フラグで明示的に選択可能にする (後方互換性)
5. `INTERFACE-COREHIR.md` の記述と `compile()` の挙動が一致すること

## 背景

- `INTERFACE-COREHIR.md`: CoreHIR は "the single source of truth for MIR lowering"
- `docs/current-state.md`: パイプライン目標 `Check+BuildCoreHIR → LowerToMIR`
- `run_frontend()` の二重 lower はコンパイル時間の観点でも無駄 (→ Issue #126 で対処)

## 参照

- `crates/ark-driver/src/session.rs:410`
- `crates/ark-driver/src/session.rs:316-340` (run_frontend — 二重 lower)
- `INTERFACE-COREHIR.md`
