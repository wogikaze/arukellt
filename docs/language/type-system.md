# 型システム

> **Normative**: This document defines the authoritative behavior of Arukellt as implemented.
> Behavior described here is verified by the fixture harness. Changes require spec review.
> For current verified state, see [../current-state.md](../current-state.md).
Wasm GC 前提の設計話や古い slice/capability 前提の説明は外し、今の利用者目線で整理しています。

## 基本型

| 型 | 説明 |
|----|------|
| `i32` | 32-bit 整数 |
| `i64` | 64-bit 整数 |
| `f32` | 32-bit 浮動小数点 |
| `f64` | 64-bit 浮動小数点 |
| `bool` | 真偽値 |
| `char` | 文字 |
| `()` | unit |
| `String` | 文字列 |

## 複合型

### struct

```ark
struct Point {
    x: i32,
    y: i32,
}
```

### enum

```ark
enum Option<T> {
    None,
    Some(T),
}
```

```ark
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

### tuple

```ark
let pair: (i32, String) = (42, String_from("hello"))
let (x, y) = pair
```

### array / Vec

```ark
let arr: [i32; 3] = [1, 2, 3]
let v: Vec<i32> = Vec_new_i32()
```

- 固定長は配列 `[T; N]`
- 可変長は `Vec<T>`
- 古い docs に出てくる slice API は現行利用の基準にはしないでください

## ジェネリクス

```ark
fn identity<T>(x: T) -> T {
    x
}
```

現行ブランチでは:

- generic functions
- generic structs
- nested generics
- trait bounds

まで実装が進んでいます。

## Option / Result

```ark
let x: Option<i32> = Some(42)
let y: Result<i32, String> = Ok(42)
```

よく使う helper:

```ark
unwrap(x)
unwrap_or(x, 0)
is_some(x)
is_none(x)
```

## 型推論

```ark
let x = 42
let s = String_from("hello")
let v: Vec<i32> = Vec_new_i32()
```

- 数値や文字列は素直な型推論が入ります
- 空コレクションは `Vec_new_T()` のような型特化コンストラクタを使う方が安全です

## 現在の実装メモ

- 実装基盤は linear memory + compiler intrinsic
- 一部 docs には Wasm GC 表現が出てきますが、それは設計資料寄りです
- いま動く型の範囲は `current-state.md` を見てください

## 関連

- [spec.md](spec.md) — 言語仕様 (凍結対象)
- [syntax.md](syntax.md)
- [memory-model.md](memory-model.md)
- [../compiler/ir-spec.md](../compiler/ir-spec.md) — CoreHIR / MIR の正規仕様
- [../current-state.md](../current-state.md)
