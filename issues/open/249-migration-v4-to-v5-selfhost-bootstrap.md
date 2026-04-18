# migration guide v4→v5: self-hosted compiler bootstrap

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-18
**ID**: 249
**Depends on**: none
**Track**: compiler/selfhost
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: Fixpoint not reached per scripts and docs/current-state.md.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

v4→v5 のセルフホスト型コンパイラ bootstrap は **未完了**（2026-04-13 の reopen 理由どおり）。
`scripts/run/verify-bootstrap.sh` は Stage 0→1→2 を順に実行するが、**Stage 2（fixpoint）は未達**
（`docs/current-state.md` の Self-Hosting Bootstrap Status 参照）。Stage 0・1 は成功している。

## Acceptance

- [ ] `scripts/run/verify-bootstrap.sh` の **full** 実行（Stage 0→1→2）が exit 0 になる（現状: Stage 2 で fixpoint 不一致のため **未達**）
- [ ] `sha256(arukellt-s1.wasm) == sha256(arukellt-s2.wasm)`（fixpoint）（**未達** — 同上）
- [x] Stage 1 が selfhost の `main.ark` 経由で wasm を生成できる（`docs/current-state.md` Stage 1 行の根拠）
- [x] v5 以降のバグ修正が Rust 実装と Arukellt 実装の両方に適用されるワークフローが確立されている（dual-period 方針として文書化済み）

### Doc alignment (2026-04-18)

`docs/migration/v4-to-v5.md` と `docs/compiler/bootstrap.md` を `scripts/run/verify-bootstrap.sh` および `docs/current-state.md` に揃えた。上記チェックボックスは根拠のあるものだけ `[x]`。

**Checkbox hygiene:** Reopen の「all [x]」誤記に対応するため、**bootstrap 完了**の 2 行（full `verify-bootstrap.sh` exit 0、fixpoint 等価）は **[ ]** のまま。`[x]` は Stage 1 経路・dual-period 方針など、根拠を本文または `docs/current-state.md` で示せる項目のみ。

## User Migration Checklist

以下はユーザーコード側の対応事項：

- [x] エンドユーザーコードのソース変更は不要
- [x] （コンパイラ貢献者）`src/compiler/` のレイアウトを把握する
- [x] （コンパイラ貢献者）fixpoint チェックには `scripts/run/verify-bootstrap.sh` を使う
- [x] （コンパイラ貢献者）バグ修正は Rust 版と Arukellt 版の両方に適用する
- [x] （コンパイラ貢献者）変更前に `docs/language/spec.md` の凍結仕様を確認する

## Scope

### Bootstrap 完成

- `scripts/run/verify-bootstrap.sh` の Stage 2（fixpoint）が **PASS** し、full 実行が exit 0 になること（現状 Stage 2 は実行されるが **FAIL**）
- 自己完結したコンパイラとして fixture / parity など広い意味での完了は `docs/compiler/bootstrap.md` と関連 issue（例 #253, #266）を参照

### Fixture 差分の解消

- Stage 0（Rust）と Stage 1（Arukellt）で fixture 出力が一致していないケースの特定と修正

## References

- `src/compiler/`
- `scripts/run/verify-bootstrap.sh`
- `docs/language/spec.md`
- `docs/current-state.md`
