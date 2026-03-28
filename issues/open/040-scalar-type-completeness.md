# Scalar 型完全化: u8/u16/u32/u64/i8/i16/f32

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 040
**Depends on**: —
**Track**: stdlib
**Blocks v3 exit**: yes

## Summary

Arukellt の scalar 型セットを `i32/i64/f64/bool/char` から拡張し、
`u8/u16/u32/u64/i8/i16/f32` を追加する。Bytes/endian/LEB128/Wasm binary
操作に unsigned 幅付き整数が不可欠であり、std::bytes と std::wasm の前提条件。

## 背景

現在 `ark-typecheck/src/types.rs` は `I32/I64/F32/F64/Bool/Char` を持つが、
unsigned 型は未定義。std.md §6 は「scalar set の拡張は optional ではない」と明記。
Wasm 自体が u8/u16/u32/u64 を要求するため、Bytes 操作や LEB128 に必須。

## 受け入れ条件

1. 型システムに `U8`, `U16`, `U32`, `U64`, `I8`, `I16`, `F32` を追加
2. リテラル表記: `42u8`, `1000u32`, `0xFFu8` 等の suffix リテラル
3. 型間の明示変換関数: `u8_to_i32`, `i32_to_u8`, `u32_to_u64` 等
4. 算術演算: `+`, `-`, `*`, `/`, `%`, 比較演算が全 scalar 型で動作
5. Wasm backend: 適切な Wasm 型にマッピング (u8/u16/i8/i16 → i32 に narrowing)
6. fixture 10 件以上

## 実装タスク

1. `ark-typecheck/src/types.rs`: 7 つの新 scalar 型バリアントを追加
2. `ark-parser`: suffix リテラル構文 (`42u8`, `1000u32`) のパース
3. `ark-typecheck/src/checker.rs`: 新 scalar 型の型検査ルール、暗黙変換の禁止
4. `ark-mir`: MIR に新 scalar 型の表現を追加
5. `ark-wasm/src/emit`: T3 emitter で u8/u16/i8/i16 を i32 として emit し、
   masking/sign-extension 命令を適切に挿入
6. `std/prelude.ark`: 変換関数 (`u8_to_i32`, `i32_to_u8` 等) を追加
7. オーバーフロー動作の定義: wrapping (Wasm の自然動作に従う)

## 検証方法

- fixture: `scalar/u8_basic.ark`, `scalar/u32_arithmetic.ark`, `scalar/u64_overflow.ark`,
  `scalar/i8_sign.ark`, `scalar/f32_basic.ark`, `scalar/conversion.ark`,
  `scalar/u8_literal.ark`, `scalar/narrow_mask.ark`, `scalar/comparison.ark`,
  `scalar/mixed_error.ark` (diag)
- 既存 fixture の regression なし

## 完了条件

- 7 つの新 scalar 型がパース→型検査→MIR→Wasm で一貫して動作する
- 変換関数が prelude に存在し、暗黙変換は起きない
- fixture 10 件以上 pass

## 注意点

1. 暗黙の型昇格を絶対に入れない — 明示変換のみ。LLM が型を見失うリスクを防ぐ
2. u8/u16 は Wasm level では i32 だが、masking が必要 (0xFF, 0xFFFF)
3. f32 の精度損失について warning を出すか検討 (f64 → f32 変換時)
4. **Wasm GC spec で `i8`/`i16` は `packedtype` として struct/array フィールドにのみ存在する**
   (`storagetype ::= valtype | packedtype`, `packedtype ::= i8 | i16`)。
   関数パラメータ・ローカル変数・戻り値としての `i8`/`i16` は Wasm の valtype に存在しない。
   Arukellt の `i8`/`i16` 型は、コンパイラ内では全て `i32` として emit し、masking/sign-extension
   で semantics を再現する。GC struct/array フィールドに格納する場合のみ packed 型を使える。
   この区別を T3 emitter の実装コメントに明記すること。

## 次版への受け渡し

- この issue で追加した unsigned 型は std::bytes (043), std::wasm (053) の直接の前提
- LEB128 codec は u32/u64 を入力に取るため、この issue が先行必須

## ドキュメント

- `docs/spec/scalar-types.md`: 全 scalar 型の一覧、Wasm マッピング、変換規則
- `std/manifest.toml` への新型・新関数の追加

## 未解決論点

1. `usize`/`isize` を入れるか (Wasm では i32 相当だが意味論が異なる)
2. `i128`/`u128` は v3 では非目標とするが、将来の拡張点を残す
3. hex リテラル (`0xFF`) は全 integer 型で有効にするか、u8 のみか
