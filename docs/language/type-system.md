# 型システム

> **Normative**: This document defines the authoritative behavior of Arukellt as implemented.
> This document is intended to describe implemented behavior. Content accuracy is manually reviewed unless a section links to a specific fixture or check ID.
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

<!-- fixture: arrays/array_literal.ark --><!-- fixture: spec_examples/type_annotations.ark -->
```ark
let arr = [1, 2, 3]
let v: Vec<i32> = Vec::new<i32>()
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

impl の型パラメータは `impl<T>` の明示 binder によってのみ導入されます
（[ADR-089](../adr/ADR-089-language-v2-explicit-impl-type-parameter-binders.md)、
[spec.md §6.5](spec.md#65-impl-block)）。binder のない impl head で
単一大文字名を書いても暗黙の型パラメータにはなりません。
実装状態: binder のない impl head の単一大文字名は通常の型名として解決され、
宣言済みの型なら impl 対象になります。0.x の ADR-014 移行窓では、宣言のない
単一大文字名を impl 型パラメータとして推論し、binder 形を示す W0222 を出します。
`--strict` で窓を閉じると E0222 になり、1.0.0 で E0222 が既定になります（#946）。

## Option / Result

Option と Result の型定義は [spec.md §2.2](spec.md#22-composite-types)、
API 一覧は [spec.md §9.10–9.11](spec.md#910-option-prelude) を参照してください。
エラー処理パターンの詳細は [error-handling.md](error-handling.md) を参照。

よく使う helper:

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
o.unwrap()
o.unwrap_or(0)
o.is_some()
o.is_none()
```

## 型推論

型推論の正規ルールは [spec.md §2.5 Type Inference](spec.md#25-type-inference) を参照してください。

実用的な注意点:

- 数値や文字列は素直な型推論が入ります
- 空コレクションは `Vec::new<T>()` のような明示的な型引数付きコンストラクタを使う方が安全です

`let` の多相化は、immutable binding の literal、closure、または既に generalized された
immutable binding の alias に限られます。
call、aggregate、block、`let mut` は value restriction により monomorphic に保たれます。
完全な規則と environment の型変数を量化しない理由は [spec.md §2.8](spec.md#28-let-generalization-typescheme) と
[ADR-070](../adr/ADR-070-language-v2-let-generalization.md) を参照してください。

## 値型と参照型

値型・参照型の分類は [spec.md §2.4 Value vs Reference Semantics](spec.md#24-value-vs-reference-semantics) を参照してください。
実装上のメモリ表現については [memory-model.md](memory-model.md) を参照。
関数呼び出しは非 `Copy` 実引数を呼び出しの間だけ借用し、パラメータ経由の
mutation には `mut` パラメータが必要です
（[spec.md §3.5.1](spec.md#351-argument-passing-and-parameter-modes)、ADR-088）。
現行コンパイラではまだ強制されていません（#951）。

## 現在の実装メモ

- primary ターゲットは `wasm32-gc` で、GC オブジェクトへ lowering します
  （現在は partial。ADR-002 / ADR-013 / ADR-035、詳細は [memory-model.md](memory-model.md)）
- `wasm32` は同じ言語意味論を linear memory へ lowering する supported ターゲットです (ADR-075)
- いま動く型の範囲は `current-state.md` を見てください

## 関連

- [spec.md](spec.md) — 言語仕様 (正規リファレンス、§2 Type System)
- [syntax.md](syntax.md) — 構文要約
- [error-handling.md](error-handling.md) — エラー処理 (Result/Option の使い方)
- [memory-model.md](memory-model.md) — メモリモデル (値型・参照型の実装)
- [../compiler/ir-spec.md](../compiler/ir-spec.md) — CoreHIR / MIR の正規仕様
- [../current-state.md](../current-state.md)
