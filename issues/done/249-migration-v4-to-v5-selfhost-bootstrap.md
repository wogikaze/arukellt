# migration guide v4→v5: self-hosted compiler bootstrap

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-22
**ID**: 249
**Depends on**: none
**Track**: compiler/selfhost
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none

## Reopened by audit — 2026-04-13

**Reason**: Fixpoint not reached per scripts and docs/current-state.md.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

v4→v5 のセルフホスト型コンパイラ bootstrap は 2026-04-22 の close gate で完了確認済み。
`scripts/run/verify-bootstrap.sh` は Stage 0→1→2 を順に実行し、Stage 2 の
`sha256(arukellt-s2.wasm) == sha256(arukellt-s3.wasm)` fixpoint まで exit 0 で通過した。

## Acceptance

- [x] `scripts/run/verify-bootstrap.sh` の **full** 実行（Stage 0→1→2）が exit 0 になる（2026-04-22 close gate: exit 0）
- [x] `sha256(arukellt-s2.wasm) == sha256(arukellt-s3.wasm)`（fixpoint）（2026-04-22 close gate: `3a0350371f9dbc37becef03efffa8d20b90827161a0d9fab97163a19de341f2c`）
- [x] Stage 1 が selfhost の `main.ark` 経由で wasm を生成できる（`docs/current-state.md` Stage 1 行の根拠）
- [x] v5 以降のバグ修正が Rust 実装と Arukellt 実装の両方に適用されるワークフローが確立されている（dual-period 方針として文書化済み）

### Doc alignment (2026-04-18)

`docs/migration/v4-to-v5.md` と `docs/compiler/bootstrap.md` を `scripts/run/verify-bootstrap.sh` および `docs/current-state.md` に揃えた。上記チェックボックスは根拠のあるものだけ `[x]`。

**Progress note (docs only, 2026-04-18):** 両ドキュメントに **#499**（セルフホスト closure 構文パーサー等、例: commit `6610945`）への「Related progress」を追記していた。2026-04-22 の close gate までは fixpoint 到達の主張はなかった。

**Checkbox hygiene:** Reopen の「all [x]」誤記に対応するため、**bootstrap 完了**の 2 行（full `verify-bootstrap.sh` exit 0、fixpoint 等価）は close gate まで **[ ]** のままだった。2026-04-22 の executable verification により両方を **[x]** に更新した。

## User Migration Checklist

以下はユーザーコード側の対応事項：

- [x] エンドユーザーコードのソース変更は不要
- [x] （コンパイラ貢献者）`src/compiler/` のレイアウトを把握する
- [x] （コンパイラ貢献者）fixpoint チェックには `scripts/run/verify-bootstrap.sh` を使う
- [x] （コンパイラ貢献者）バグ修正は Rust 版と Arukellt 版の両方に適用する
- [x] （コンパイラ貢献者）変更前に `docs/language/spec.md` の凍結仕様を確認する

## Scope

### Bootstrap 完成

- `scripts/run/verify-bootstrap.sh` の Stage 2（fixpoint）が **PASS** し、full 実行が exit 0 になること（2026-04-22 close gate で達成）
- 自己完結したコンパイラとして fixture / parity など広い意味での完了は `docs/compiler/bootstrap.md` と関連 issue（例 #253, #266）を参照

### Fixture 差分の解消

- Stage 0（Rust）と Stage 1（Arukellt）で fixture 出力が一致していないケースの特定と修正

## References

- `src/compiler/`
- `scripts/run/verify-bootstrap.sh`
- `docs/language/spec.md`
- `docs/current-state.md`

## Close note (2026-04-22)

Closed by verification-only close gate on master. Evidence baseline commit before
closure: `ee1e3b4a6457aa5cf423822400af23e6a61d1d28`.

Acceptance mapping:

1. Full `scripts/run/verify-bootstrap.sh` Stage 0→1→2 gate is satisfied:
   command exited 0. Stage 0 produced `arukellt-s1.wasm` (570743 bytes),
   Stage 1 produced `arukellt-s2.wasm` (536277 bytes), and Stage 2 produced
   `arukellt-s3.wasm` (536277 bytes).
2. Fixpoint is satisfied by the executable Stage 2 digest check:
   `sha256(arukellt-s2.wasm) == sha256(arukellt-s3.wasm) ==
   3a0350371f9dbc37becef03efffa8d20b90827161a0d9fab97163a19de341f2c`.
3. Stage 1 remains satisfied: Stage 1 compiled the selfhost `main.ark` path via
   `arukellt-s1.wasm` and produced `arukellt-s2.wasm`.
4. The dual-period workflow acceptance remains satisfied by the existing issue
   text and repository process; no product behavior changed in this close-only
   slice.

Verification results:

```text
bash scripts/run/verify-bootstrap.sh                         # exit 0
python3 scripts/manager.py selfhost fixpoint                 # exit 0
python3 scripts/manager.py selfhost fixture-parity           # exit 0
python3 scripts/manager.py selfhost parity --mode --cli      # exit 0
python3 scripts/manager.py selfhost diag-parity              # exit 0
```

Canonical gate deltas from wave baseline:

```text
fixture parity: PASS=302 (Δ+0) FAIL=0 (Δ+0) SKIP=47 (Δ+0)
diag parity:    PASS=12 (Δ+0) FAIL=0 (Δ+0) SKIP=22 (Δ+0)
cli parity:     PASS=6 (Δ+0) FAIL=0 (Δ+0)
fixpoint:       rc=0
```
