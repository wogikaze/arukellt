# 構文仕様

> **Normative**: This document defines the authoritative behavior of Arukellt as implemented.
> Behavior described here is verified by the fixture harness. Changes require spec review.
> For current verified state, see [../current-state.md](../current-state.md).

このページは、現行ブランチで把握しやすい構文の **実用要約** です。
設計-only の capability I/O や古い v0 制約の説明は落とし、現在よく使う書き方を優先しています。

> **正規仕様との関係**: 構文の完全な定義は [spec.md](spec.md) (§1 Lexical Structure, §3–6 Expressions/Statements/Items, Appendix A Grammar) を参照してください。
> 型については [type-system.md](type-system.md)、エラー処理については [error-handling.md](error-handling.md) が各トピックの正規リファレンスです。

## エントリポイント

```ark
fn main() {
}
```

```ark
fn main() -> i32 {
    0
}
```

## import

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
import math
import utils as u
```

- 基本形は `import <name>`
- alias 付きは `import <name> as <alias>`
- qualified access は `math::add(1, 2)` の形を使います
- capability 引数付き `main(caps: ...)` は現行の一般的 API ではありません

> 📘 import/use の完全な構文は [spec.md §7 Module System](spec.md#7-module-system) を参照。

## 変数と関数

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let x = 42
let mut y = 0

y = y + 1

fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

> 📘 関数定義・let 束縛の完全な仕様は [spec.md §4.1 Let Binding](spec.md#41-let-binding), [§6.1 Function Definition](spec.md#61-function-definition) を参照。

## 型アノテーション

型の一覧・構造体・enum の正規定義は [type-system.md](type-system.md) にあります。
基本的な型アノテーションの書き方は以下の通りです:

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let n: i32 = 42
let s: String = String_from("hello")
```

> 📘 基本型・複合型の完全な一覧は [spec.md §2 Type System](spec.md#2-type-system) を参照。
> struct / enum の定義構文は [spec.md §6.2–6.3](spec.md#62-struct-definition) を参照。

## 制御構文

### if

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let label = if x > 0 {
    String_from("positive")
} else {
    String_from("other")
}
```

### while / loop

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
while x < 10 {
    x = x + 1
}

loop {
    if done {
        break
    }
}
```

### for

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
for i in 0..10 {
    println(to_string(i))
}

for item in values(v) {
    println(to_string(item))
}
```

> 📘 制御構文の完全な仕様は [spec.md §3.10–3.18](spec.md#310-if-expression), [§4.3–4.5](spec.md#43-while-loop) を参照。

## 関数呼び出しスタイル

共通で安全なのは関数呼び出し形式です。

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
push(v, 42)
let n = len(v)
let s2 = concat(s1, s2)
let text = to_string(n)
```

このブランチでは v1 のメソッド構文もありますが、まずは上の形を基準にするのが安全です。
文字列化も `.to_string()` より `to_string(x)` を基準にするのが安全です。

## match

パターンマッチの基本的な使い方:

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
match value {
    0 => String_from("zero"),
    1 => String_from("one"),
    _ => String_from("other"),
}
```

> 📘 パターンの種類 (wildcard, or-pattern, struct pattern, guard 等) は [spec.md §5 Pattern Matching](spec.md#5-pattern-matching) を参照。
> Option/Result のマッチについては [error-handling.md](error-handling.md) を参照。

## v1 実装済み構文

このブランチでは次も入っています。

- `trait`
- `impl`
- メソッド呼び出し
- 演算子オーバーロード
- match guard / or-pattern / struct pattern
- nested generics

詳細は [syntax-v1-preview.md](syntax-v1-preview.md) を参照してください。

## 関連

- [spec.md](spec.md) — 言語仕様 (正規リファレンス)
- [type-system.md](type-system.md) — 型システム
- [error-handling.md](error-handling.md) — エラー処理
- [memory-model.md](memory-model.md) — メモリモデル
- [../quickstart.md](../quickstart.md)
- [../current-state.md](../current-state.md)
