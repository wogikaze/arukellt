# `MIRValidate` をコンパイル経路で無条件実行

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 127
**Depends on**: —
**Track**: pipeline-refactor
**Blocks v4 exit**: no (quality gate)


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/127-mir-validate-unconditional.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

計画パイプライン `LowerToMIR → MIRValidate → MIROptimize` では `MIRValidate` は
`MIROptimize` の前に常に実行されるべきゲートである。しかし現在:

- `validate_mir()` は `run_frontend()` で Legacy パスのみ呼ばれる
- `compile_selected()` では `OptimizedLegacy`/`OptimizedCoreHir` のときのみ呼ばれる
- 最適化なし (`MirSelection::CoreHir`) での emit 前に validate が走らない

```rust
// compile_selected() の non-optimized path → validate_mir 呼び出しなし
if matches!(selection, MirSelection::OptimizedLegacy | MirSelection::OptimizedCoreHir) {
    optimize_module(&mut mir)?;
    validate_mir(&mir)?;  // 最適化後のみ
}
```

## 受け入れ条件

1. `compile_selected()` で選択に関わらず `validate_mir()` を lower 直後に実行
2. `validate_backend_ready_mir()` は CoreHIR パスでのみ (現行通り)
3. `scripts/run/verify-harness.sh` が status 0 で終了
4. パイプライン図の `MIRValidate` ステージと `ark-mir/src/validate.rs` の内容が一致すること
5. validate 失敗時は `E0501` 診断コードを emit (既存か確認)

## 背景

- MIR validation は `ark-mir/src/validate.rs` に実装済み
- 無条件実行によりバックエンドへの不正 MIR 流入を防ぐ
- コンパイル時間への影響は小さい (構造検査のみ)

## 参照

- `crates/ark-driver/src/session.rs:438-470` (compile_selected)
- `crates/ark-mir/src/validate.rs`
- `docs/current-state.md` (パイプライン仕様)
