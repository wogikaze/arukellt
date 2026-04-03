# std::core: Error 型、ordering、range、cmp、math、convert、hash

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 041
**Depends on**: 039
**Track**: stdlib
**Blocks v3 exit**: yes


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/041-std-core.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

std::core モジュールとして、言語の基礎型と基礎関数を体系化する。
Error enum の標準化、Ordering/Range 型、比較・数学・変換・ハッシュ関数を整備し、
後続の全 stdlib モジュールが依存する共通基盤を確立する。

## 受け入れ条件

### Error 型

```ark
pub enum Error {
    InvalidArgument(String),
    IndexOutOfBounds { index: i32, len: i32 },
    ParseError { kind: String, input: String },
    Utf8Error,
    IoError(String),
    NotFound(String),
    AlreadyExists(String),
    PermissionDenied(String),
    Timeout,
    WasmError(String),
    ComponentError(String),
}
pub type StdResult<T> = Result<T, Error>
```

### Ordering / Range

```ark
pub enum Ordering { Less, Equal, Greater }
pub struct Range { start: i32, end: i32 }
pub struct RangeInclusive { start: i32, end: i32 }
```

### 主要関数

- `cmp::min<T>(a: T, b: T) -> T`, `cmp::max<T>`, `cmp::clamp<T>`
- `math::abs_i32`, `math::abs_i64`, `math::abs_f64`, `math::pow_f64`,
  `math::sqrt`, `math::floor`, `math::ceil`, `math::round`, `math::log`, `math::exp`
- `convert::i32_to_string`, `convert::parse_i32`, `convert::f64_to_string` (既存の再配置)
- `hash::hash_i32`, `hash::hash_string`, `hash::combine`

## 実装タスク

1. `std/core/error.ark`: Error enum 定義、display/message 関数
2. `std/core/ordering.ark`: Ordering enum、compare 関数群
3. `std/core/range.ark`: Range/RangeInclusive 型、contains/len 関数
4. `std/core/math.ark`: 数学関数 (既存 intrinsic の再配置 + 新規追加)
5. `std/core/convert.ark`: 型変換関数の統一 namespace 化
6. `std/core/hash.ark`: ハッシュ関数 (HashMap の前提)
7. `std/manifest.toml` を更新

## 検証方法

- fixture: `stdlib_core/error_basic.ark`, `stdlib_core/ordering.ark`,
  `stdlib_core/range.ark`, `stdlib_core/math.ark`, `stdlib_core/convert.ark`,
  `stdlib_core/hash.ark`, `stdlib_core/error_match.ark`
- 既存の `stdlib_math/` fixture との重複解消

## 完了条件

- Error enum が定義され、match で分岐できる
- Ordering, Range 型が使える
- math/convert/hash 関数がモジュール import で呼び出せる
- fixture 7 件以上 pass

## 注意点

1. Error enum のバリアント数を爆発させない — v3 では上記 11 バリアントを上限とする
2. 既存の `i32_to_string` 等を即座に削除しない — prelude から deprecation 警告を出しつつ共存
3. hash 関数の実装品質: FNV-1a または wyhash 相当の簡易ハッシュから始める

## 次版への受け渡し

- Error 型は全 stdlib モジュールのエラー返却の標準形になる
- hash 関数は std::collections::hash_map (044) の直接の前提

## ドキュメント

- `docs/stdlib/core-reference.md`: Error, Ordering, Range, math, convert, hash の API リファレンス

## 未解決論点

1. `Error` に `Custom(String)` バリアントを入れるか
2. `StdResult<T>` を prelude に入れるか、明示 import を要求するか
3. `math::PI`, `math::E` 等の定数をどう表現するか (const 構文の有無に依存)
