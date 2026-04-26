# エラー処理

> **Normative**: This document defines the authoritative behavior of Arukellt as implemented.
> Behavior described here is verified by the fixture harness. Changes require spec review.
> For current verified state, see [../current-state.md](../current-state.md).

このページはエラー処理の **実用ガイド** です。基本的に `Result<T, E>` と `Option<T>` ベースです。
古い docs にある capability I/O 専用エラー階層は、現行 API の基準ではありません。

> **正規仕様との関係**: `?` 演算子は [spec.md §3.9 Try Operator](spec.md#39-try-operator)、
> Option API は [spec.md §9.10](spec.md#910-option)、Result API は [spec.md §9.11](spec.md#911-result)、
> panic は [spec.md §9.4](spec.md#94-control) を参照してください。

## 基本方針

- 例外ベースではない
- 通常の失敗は `Result<T, E>`
- 値がない正常ケースは `Option<T>`
- `panic` は回復不能な場面向け

> 📘 Option/Result の型定義は [spec.md §2.2 Composite Types](spec.md#22-composite-types)、
> 利用可能な関数の完全な一覧は [spec.md §9.10–9.11](spec.md#910-option) を参照。

## `?` 演算子

`expr?` は `Err` variant を自動的に伝搬します。
囲んでいる関数は `Result<_, E>` を返す必要があります。

```ark
fn parse_twice(s: String) -> Result<i32, String> {
    let n = parse_i32(s)?
    Ok(n * 2)
}
```

> 📘 `?` 演算子の正規仕様は [spec.md §3.9 Try Operator](spec.md#39-try-operator) を参照。

このブランチでは v1 系の拡張も入っていますが、まずは `Result<_, String>` を基準に考えるのが分かりやすいです。

## panic

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
panic(String_from("unreachable"))
```

- 現行実装では panic は stderr 出力 + trap です
- 通常フローに多用する前提ではありません

## 現在よく見るエラー型

現行 stdlib wrapper では、たとえば以下のような形が多いです。

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
Result<String, String>
Result<(), String>
```

設計文書上の `IOError` などは将来設計として読むべきで、現行 API の前提にはしないでください。

## 関連

- [spec.md](spec.md) — 言語仕様 (§3.9 Try Operator, §9.10–9.11 Option/Result API)
- [type-system.md](type-system.md) — 型システム (Option/Result の型定義)
- [../compiler/diagnostics.md](../compiler/diagnostics.md)
- [../stdlib/io.md](../stdlib/io.md)
- [../current-state.md](../current-state.md)
