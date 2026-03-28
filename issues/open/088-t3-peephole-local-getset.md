# T3 Peephole: local.get/set 冗長ペア除去

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 088
**Depends on**: —
**Track**: backend-opt
**Blocks v4 exit**: yes

## Summary

T3 emitter が生成する Wasm 命令列に対して、
`local.set X` の直後に `local.get X` (他の命令を挟まない) が続くパターンを
スタック値をそのまま使う形に変換する peephole 最適化を追加する。
roadmap-v4.md §5.3 で明示的に要求されている最適化。

## 対象パターン

```wasm
local.set $x    ;; スタックの値を $x に保存
local.get $x    ;; すぐ読み戻す
→ (削除: スタック値をそのまま次命令に渡す)
```

## 受け入れ条件

1. `ark-wasm/src/emit/t3_wasm_gc.rs` に `peephole_local_getset()` 関数追加
2. 命令バッファをポストプロセスして上記パターンを除去
3. `hello.wasm` のバイナリサイズ削減を `wc -c` で確認
4. 全 fixture が peephole あり/なしで同じ出力を生成することを確認

## 実装タスク

1. `emit()` 完了後に命令バッファを走査して peephole 適用
2. `--opt-level 0` では peephole 無効 (デバッグ用)

## 参照

- `docs/process/roadmap-v4.md` §5.3
