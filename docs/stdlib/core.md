# std/core — 現在使えるコア API

> このページは v2 以前の旧インデックスです。  
> **v3 以降は [modules/core.md](modules/core.md) を参照してください。**

## リダイレクト

- Option / Result → [modules/core.md](modules/core.md)
- Vec / HashMap → [modules/collections.md](modules/collections.md)
- String → [modules/text.md](modules/text.md)
- 安定性ラベル付き全 API → [reference.md](reference.md)


現在よく使う公開面:

```ark
Some(x)
None
Ok(x)
Err(e)
unwrap(x)
unwrap_or(x, default)
is_some(x)
is_none(x)
map_option_i32_i32(opt, f)
```

メモ:

- `map_option_i32_i32` は存在します
- `unwrap_or_else` や汎用 `ok_or` など、設計文書上の API は現状未確認です

## Vec

現在確認できる主な API:

```ark
Vec_new_i32()
Vec_new_i64()
Vec_new_f64()
Vec_new_String()
push(v, x)
pop(v)
get(v, i)
get_unchecked(v, i)
set(v, i, x)
len(v)
```

追加で使える helper / HOF:

```ark
sort_i32(v)
sort_i64(v)
sort_f64(v)
sort_String(v)
map_i32_i32(v, f)
filter_i32(v, f)
fold_i32_i32(v, init, f)
map_i64_i64(v, f)
filter_i64(v, f)
fold_i64_i64(v, init, f)
map_f64_f64(v, f)
filter_f64(v, f)
map_String_String(v, f)
filter_String(v, f)
any_i32(v, f)
find_i32(v, f)
contains_i32(v, x)
contains_String(v, x)
reverse_i32(v)
reverse_String(v)
remove_i32(v, index)
sum_i32(v)
product_i32(v)
```

## String

現在確認できる主な API:

```ark
String_from(s)
String_new()
eq(a, b)
concat(a, b)
clone(s)
slice(s, start, end)
split(s, delim)
join(parts, sep)
starts_with(s, prefix)
ends_with(s, suffix)
to_lower(s)
to_upper(s)
push_char(s, c)
```

## 変換・数学・制御

```ark
i32_to_string(n)
i64_to_string(n)
f64_to_string(n)
bool_to_string(b)
char_to_string(c)
parse_i32(s)
parse_i64(s)
parse_f64(s)
sqrt(x)
abs(x)
min(a, b)
max(a, b)
clamp_i32(x, lo, hi)
panic(msg)
```

## Assert

```ark
assert(cond)
assert_eq(a, b)
assert_ne(a, b)
assert_eq_i64(a, b)
assert_eq_str(a, b)
```

## いまは設計だけのもの

以下は古い設計文書には出てきますが、このページでは現行 API として扱いません。

- Wasm GC 前提の型表現
- `slice[T]` API 群
- `mem_copy` / `mem_set` などの低レベル `mem` モジュール
- 汎用 trait 前提のコレクション API
- capability I/O 型

## 関連

- [標準ライブラリ概要](README.md)
- [Cookbook](cookbook.md)
- [Current state](../current-state.md)
