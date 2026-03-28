# MIR: LICM (ループ不変式移動) パス

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 080
**Depends on**: —
**Track**: mir-opt
**Blocks v4 exit**: yes

## Summary

`crates/ark-mir/src/opt/` に `licm.rs` (Loop Invariant Code Motion) パスを追加する。
ループ本体内で変化しない計算をループ前に移動することで、
`vec-ops` (10万要素ループ) などのベンチマークで実行時間を削減する。
roadmap-v4.md §5.2 item 5 で明示的に要求されているパス。

## 対象パターン

- ループ内で使われるが全ループイテレーションで同じ値になる `BinOp` / `UnaryOp`
- ループ不変な struct フィールド読み取り (`struct.get` の結果が一定)
- 配列長 (`array.len`) のループ内毎回計算

## 受け入れ条件

1. `passes/licm.rs`: ループ内の不変計算を pre-header ブロックに移動
2. `OptimizationPass::Licm` を enum に追加・`DEFAULT_PASS_ORDER` に挿入
3. ループのネストに対応 (最も外側のループへの移動を優先)
4. `--opt-level 2` でのみ有効
5. `vec_push_pop.ark` ベンチマークで `--opt-level 1` 比 15% 以上改善
6. 副作用のある呼び出し (I/O 等) は移動しない

## 実装タスク

1. `ark-mir/src/opt/licm.rs`: CFG からループ検出 (支配木 + back-edge)
2. pre-header ブロック生成・不変命令の移動
3. `OptimizationSummary` に `licm_hoisted: usize` 追加

## 参照

- `docs/process/roadmap-v4.md` §5.2 item 5
