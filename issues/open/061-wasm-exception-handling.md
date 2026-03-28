# Wasm native 例外処理: try_table / throw / exnref 実装

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 061
**Depends on**: —
**Track**: wasm-feature
**Blocks v4 exit**: no

**Status note**: Wasm proposal — deferred to v5+. Not implemented.

## Summary

WebAssembly 3.0 例外処理提案 (`docs/spec/spec-3.0.0/proposals/exception-handling/Exceptions.md`) の
`tag` セクション・`throw` / `try_table` / `catch` / `exnref` を T3 emitter で直接使用する。
現状の `TryExpr` (`?` 演算子) は Wasm native 例外でなく GC ヒープ経由のエラー伝播で実装されており、
Wasm native 例外に切り替えることで `?` チェーンのオーバーヘッドをほぼゼロにできる。

## 背景

現在の `?` 演算子は `anyref` スクラッチローカル経由で Result 値を伝播している（`TryExpr` ハンドリング）。
Wasm native の `throw` / `try_table` は専用のハードウェア機構に委譲されるため、
エラーパスが遅いコード (99% は成功パス) での実行速度が大幅に向上する。

## 受け入れ条件

1. `ark-wasm` に `tag` セクション生成を追加 (Result::Err 用の exception tag)
2. `?` 演算子の成功パスを `throw_ref` / `try_table` に変換するオプション (`--opt-level 2`)
3. `exnref` を catch し Ark の `Result::Err` 値として再構築
4. wasmtime が exception handling をサポートしていることを確認
5. `?` 演算子を多用するパターンのベンチマークで 10% 以上の実行時間改善

## 実装タスク

1. `ark-wasm/src/emit/t3_wasm_gc.rs`: TagSection 追加、`throw`/`try_table` emit
2. `ark-mir/src/mir.rs`: `MirTerminator::ThrowIfErr` 追加
3. `ark-mir/src/lower.rs`: `?` 演算子を `ThrowIfErr` に変換
4. `tests/fixtures/opt/exception_chain.ark`: `?` を 10 段ネストした fixture

## 参照

- `docs/spec/spec-3.0.0/proposals/exception-handling/Exceptions.md`
