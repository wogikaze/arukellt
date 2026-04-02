# 型システム

> **Normative**: This document defines the authoritative behavior of Arukellt as implemented.
> Behavior described here is verified by the fixture harness. Changes require spec review.
> For current verified state, see [../current-state.md](../current-state.md).

このページは型システムの **実用ガイド** です。
Wasm GC 前提の設計話や古い slice/capability 前提の説明は外し、今の利用者目線で整理しています。

> **正規仕様との関係**: 型の完全な定義 (基本型・複合型・ジェネリクス・型推論・型変換ルール) は
> [spec.md §2 Type System](spec.md#2-type-system) を参照してください。
> このページでは、仕様を補足する実用的な注意点を中心に記載しています。

## 基本型

基本型の完全な一覧と Wasm マッピングは [spec.md §2.1 Primitive Types](spec.md#21-primitive-types) を参照してください。

よく使う基本型の概要:

| 型 | 説明 |
|----|------|
| `i32` | 32-bit 整数 (デフォルト) |
| `i64` | 64-bit 整数 |
| `f64` | 64-bit 浮動小数点 (デフォルト) |
| `bool` | 真偽値 |
| `char` | 文字 |
| `()` | unit |
| `String` | 文字列 |

> 📘 `f32`, `u8`, `u16`, `u32`, `u64`, `i8`, `i16` も利用可能です。完全な一覧は [spec.md §2.1](spec.md#21-primitive-types) を参照。

## 複合型

複合型 (struct, enum, tuple, array, Vec, Option, Result 等) の定義構文と意味論は
[spec.md §2.2 Composite Types](spec.md#22-composite-types) を参照してください。

### 利用者向け実用ポイント

```ark
let arr: [i32; 3] = [1, 2, 3]
let v: Vec<i32> = Vec_new_i32()
```

- 固定長は配列 `[T; N]`
- 可変長は `Vec<T>`
- 古い docs に出てくる slice API は現行利用の基準にはしないでください

> 📘 struct/enum の定義構文は [spec.md §6.2–6.3](spec.md#62-struct-definition) を参照。

## ジェネリクス

ジェネリクスの正規仕様は [spec.md §2.7 Generics](spec.md#27-generics) を参照してください。

現行ブランチでは:

- generic functions
- generic structs
- nested generics
- trait bounds

まで実装が進んでいます。

## Option / Result

Option と Result の型定義は [spec.md §2.2](spec.md#22-composite-types)、
API 一覧は [spec.md §9.10–9.11](spec.md#910-option) を参照してください。
エラー処理パターンの詳細は [error-handling.md](error-handling.md) を参照。

よく使う helper:

```ark
unwrap(x)
unwrap_or(x, 0)
is_some(x)
is_none(x)
```

## 型推論

型推論の正規ルールは [spec.md §2.5 Type Inference](spec.md#25-type-inference) を参照してください。

実用的な注意点:

- 数値や文字列は素直な型推論が入ります
- 空コレクションは `Vec_new_T()` のような型特化コンストラクタを使う方が安全です

## 値型と参照型

値型・参照型の分類は [spec.md §2.4 Value vs Reference Semantics](spec.md#24-value-vs-reference-semantics) を参照してください。
実装上のメモリ表現については [memory-model.md](memory-model.md) を参照。

## 現在の実装メモ

- 実装基盤は linear memory + compiler intrinsic (詳細は [memory-model.md](memory-model.md))
- 一部 docs には Wasm GC 表現が出てきますが、それは設計資料寄りです
- いま動く型の範囲は `current-state.md` を見てください

## 関連

- [spec.md](spec.md) — 言語仕様 (正規リファレンス、§2 Type System)
- [syntax.md](syntax.md) — 構文要約
- [error-handling.md](error-handling.md) — エラー処理 (Result/Option の使い方)
- [memory-model.md](memory-model.md) — メモリモデル (値型・参照型の実装)
- [../compiler/ir-spec.md](../compiler/ir-spec.md) — CoreHIR / MIR の正規仕様
- [../current-state.md](../current-state.md)
