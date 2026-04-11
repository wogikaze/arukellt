# T3: MIR 最適化の段階的再開 (GC-safe audit + opt-level 復帰)

**Status**: open
**Created**: 2026-04-11
**Updated**: 2026-04-11
**ID**: 486
**Depends on**: 122
**Track**: mir-opt
**Blocks v1 exit**: no

---

## Summary

2026-04-11 の CI 修正で、`wasm32-wasi-p2` は fixture / selfhost 安定化のため
MIR 最適化だけを一時的に `O0` に固定し、backend 側の tail-call 最適化だけを残した。
これで invalid Wasm は解消したが、T3 では `--opt-level 1/2` の MIR パス群が
実質停止したままになっている。

この issue では、T3 で unsafe だった MIR パスを棚卸しし、
`TryExpr` / `Result` ラッパー / host facade / tail-call 周辺の型不整合を
再発させない形で、段階的に最適化を再開する。

## 受け入れ条件

1. `crates/ark-driver/src/session.rs` の T3 向け MIR `O0` 固定を撤去するか、少なくとも pass 単位の明示的 gate に縮小する
2. T3 で有効化する各 MIR パスについて、GC / enum / specialized `Result` / tail-call 互換性の前提条件を整理する
3. `ARUKELLT_TARGET=wasm32-wasi-p2 cargo test -p arukellt --test harness -- --nocapture` が、MIR 最適化を有効にした既定設定で通る
4. 以下の回帰再現 fixture が green のまま維持される
   - `tests/fixtures/stdlib_option_result/question_mark.ark`
   - `tests/fixtures/stdlib_io/fs_read_error.ark`
   - `tests/fixtures/stdlib_io/fs_read_write.ark`
   - `tests/fixtures/stdlib_http_compile.ark`
   - `tests/fixtures/from_trait/from_auto_convert.ark`
   - `tests/fixtures/tail_call/deep_recursion.ark`
   - `tests/fixtures/tail_call/opportunistic.ark`
5. まだ T3 で無効のままにする pass がある場合、その理由と次の解除条件を issue か docs に記録する

## 実装タスク

1. `crates/ark-mir/src/opt/pipeline.rs` の各 pass について、T3 で invalid Wasm を起こすものを再現ベースで分類する
2. `desugar` 後の typed temp locals、specialized `Result` 名、qualified host wrapper 呼び出し解決に依存する pass を洗い出す
3. pass 単位で T3 有効化を戻し、fixture harness で段階的に回帰確認する
4. 必要なら `ARUKELLT_DUMP_PHASES` / pass trace を使って、どの変換で型崩れが入るか追跡できるようにする

## 参照

- `issues/open/122-compile-mir-opt-level-separation.md`
- `crates/ark-driver/src/session.rs`
- `crates/ark-mir/src/opt/desugar.rs`
- `crates/ark-mir/src/opt/pipeline.rs`
- `crates/ark-wasm/src/emit/t3/stmts.rs`
- `crates/ark-wasm/src/emit/t3/operands.rs`
