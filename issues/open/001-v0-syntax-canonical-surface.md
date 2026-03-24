# v0 syntax surface is not canonical enough for LLM-oriented design

**Status**: open
**Created**: 2026-03-24
**Updated**: 2026-03-24

## Summary

v0 の仕様文書間で自己矛盾があり、設計思想と表面構文が一致していない。LLM が安定して書くための canonical surface が崩れている。

### 問題 1: 仕様文書間の自己矛盾

- `impl` は v1 予約キーワードなのに syntax.md の構文例で使われている
- 文字列リテラル `"hello"` に `// str` コメントがあるが、使用例は `String::from("hello")`

### 問題 2: 設計思想と表面の乖離

- ADR-004 は trait なし・演算子オーバーロードなしを明記しているのに、syntax では通常の演算子とプリミティブメソッド（`.sqrt()` など）が前面に出ている
- 「method はあるが trait はない」という半端な世界になっている

### 問題 3: 正規形が多すぎる

- `[]` の多義性: generics (`Vec[T]`)、配列型 (`[i32; 3]`)、スライス型 (`[i32]`)、添字 (`arr[0]`)
- `!` の多義性: 論理否定と整数ビット否定
- `match` の高機能化: guard, or-pattern, struct destructuring が v0 に入っている
- 重い `main` 例: `main(caps: Capabilities) -> Result[(), AppError]` が構文入門の冒頭

## Acceptance Criteria

- [ ] syntax.md 単体で v0 / v1 の境界が矛盾しない
- [ ] 文字列リテラルの型が一意に決まる（v0 では `String` に固定）
- [ ] `[]` の役割が 2 個以下に減る（generics は `<T>` に変更、`[]` は配列・スライス・添字のみ）
- [ ] `match` の v0 機能が明文化される（guard, or-pattern, struct destructuring は v1 送り）
- [ ] 最小 `main` 例 (`fn main() {}`) が最初に来る
- [ ] ADR-004 と syntax.md の演算子方針が一致する（built-in 演算子の適用可能型を列挙）

## Implementation Phases

### Phase 1: 矛盾除去

`impl` を v0 から落とす。メソッド構文も v1 送り。

- `Point::new(...)` → `point_new(...)`
- `p.distance(...)` → `point_distance(p, q)`
- プリミティブメソッド (`.sqrt()`, `.len()`) → 組み込み関数 (`sqrt(x)`, `len(s)`)

根拠: trait なし、解決規則最小、LLM フレンドリを本気でやるなら、関数呼び出しの方が安定。

### Phase 2: 文字列の一本化

v0 では `"..."` の型を `String` に固定。`str` はユーザー表面に出さない。

### Phase 3: 記号の一意性を増やす

generics を `<T>` に変更:
- `Vec<T>`, `Result<T, E>`, `Option<T>`
- `fn identity<T>(x: T) -> T`

`[]` は配列・スライス・添字のみに閉じ込める:
- 配列型: `[i32; 3]`
- スライス型: `[i32]`
- 添字: `arr[0]`

### Phase 4: match を v0 サイズに削る

v0 の `match`:
- リテラルパターン
- enum variant パターン
- ワイルドカード `_`
- 変数束縛

v1 送り:
- guard (`n if n > 0`)
- or-pattern (`1 | 2`)
- struct destructuring (`Point { x, y }`)

### Phase 5: 演算子の位置づけの明記

built-in 演算子の適用可能型を v0 で列挙:

| 演算子 | 適用可能型 |
|--------|-----------|
| `+`, `-`, `*`, `/`, `%` | `i32`, `i64`, `f32`, `f64` |
| `==`, `!=` | 全プリミティブ型, `String` |
| `<`, `<=`, `>`, `>=` | 数値型, `char` |
| `&&`, `\|\|`, `!` (論理) | `bool` |
| `&`, `\|`, `^`, `!` (ビット), `<<`, `>>` | `i32`, `i64` |

`!` の曖昧性は型で解決: `bool` なら論理否定、整数なら bit NOT。

### Phase 6: 入門面の軽量化

最小 `main` 例を冒頭に:

```
fn main() {
    // 最小のエントリポイント
}
```

`Capabilities` と `Result` は後ろの「モジュール」「エラー処理」節に移動。

## Notes

### 関連文書

- `docs/language/syntax.md`: 主な修正対象
- `docs/language/type-system.md`: generics 記法の変更
- `docs/language/memory-model.md`: String の扱いを確認
- `docs/adr/ADR-004-trait-strategy.md`: 設計方針の根拠

### 背景

LLM にとって最も重要なのは簡潔さより正規形の少なさ。「Rust っぽさを借りているが、Rust の厳密さは借りていない」状態は、LLM が既存 Rust 分布に引っ張られてもっともらしいが仕様違反のコードを書く原因になる。
