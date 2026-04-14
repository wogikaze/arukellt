# std::text: String API 拡張、StringBuilder、format ユーティリティ

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 042
**Depends on**: 039, 041
**Track**: stdlib
**Blocks v3 exit**: yes

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/042-std-text.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

現在 prelude に散在する String 操作関数を `std::text::string` モジュールに再配置し、
不足している API (from_utf8, to_utf8_bytes, len_chars, slice_bytes, lines, trim,
replace, chars, normalize) を追加する。StringBuilder と format ユーティリティも新設する。

## 背景

現在の prelude.ark には `concat`, `split`, `join`, `starts_with`, `ends_with`,
`contains_String`, `to_lower`, `to_upper`, `slice`, `push_char`, `i32_to_string` 等が
flat に定義されている。v3 ではこれらを `std::text::string` に移動し、
`string::split(s, sep)` のような修飾呼び出しで使えるようにする。

## 受け入れ条件

### string モジュール (新規 + 再配置)

```ark
// 新規
pub fn from_utf8(bytes: Bytes) -> Result<String, Error>
pub fn to_utf8_bytes(s: String) -> Bytes
pub fn len_bytes(s: String) -> i32
pub fn len_chars(s: String) -> i32
pub fn slice_bytes(s: String, start: i32, end: i32) -> Result<String, Error>
pub fn lines(s: String) -> Vec<String>
pub fn trim(s: String) -> String
pub fn trim_start(s: String) -> String
pub fn trim_end(s: String) -> String
pub fn replace(s: String, from: String, to: String) -> String
pub fn repeat(s: String, n: i32) -> String
pub fn chars(s: String) -> Vec<char>
pub fn is_empty(s: String) -> bool

// 再配置 (prelude から移動、prelude にも deprecated wrapper を残す)
pub fn split(s: String, sep: String) -> Vec<String>
pub fn join(parts: Vec<String>, sep: String) -> String
pub fn concat(a: String, b: String) -> String
pub fn starts_with(s: String, prefix: String) -> bool
pub fn ends_with(s: String, suffix: String) -> bool
pub fn contains(s: String, needle: String) -> bool
pub fn to_lower(s: String) -> String
pub fn to_upper(s: String) -> String
```

### StringBuilder

```ark
pub fn builder_new() -> StringBuilder
pub fn builder_append(sb: StringBuilder, s: String) -> StringBuilder
pub fn builder_append_char(sb: StringBuilder, c: char) -> StringBuilder
pub fn builder_append_line(sb: StringBuilder, s: String) -> StringBuilder
pub fn builder_build(sb: StringBuilder) -> String
pub fn builder_len(sb: StringBuilder) -> i32
```

### fmt ユーティリティ

```ark
pub fn format_i32(n: i32) -> String
pub fn format_f64(n: f64, precision: i32) -> String
pub fn format_bool(b: bool) -> String
pub fn pad_left(s: String, width: i32, fill: char) -> String
pub fn pad_right(s: String, width: i32, fill: char) -> String
```

## 実装タスク

1. `std/text/string.ark`: 上記 API の実装 (intrinsic + source)
2. `std/text/builder.ark`: StringBuilder (GC array の動的拡張で実装)
3. `std/text/fmt.ark`: format ユーティリティ
4. 既存 prelude の string 関数に `@deprecated` コメントを付与
5. `ark-typecheck`: StringBuilder 型の登録
6. `ark-wasm/src/emit`: StringBuilder の intrinsic (append → array concat) を実装

## 検証方法

- fixture: `stdlib_text/string_from_utf8.ark`, `stdlib_text/string_lines.ark`,
  `stdlib_text/string_trim.ark`, `stdlib_text/string_replace.ark`,
  `stdlib_text/string_chars.ark`, `stdlib_text/string_repeat.ark`,
  `stdlib_text/builder_basic.ark`, `stdlib_text/fmt_pad.ark`
- 既存 `stdlib_string/` fixture が引き続き pass する

## 完了条件

- 上記 API が `use std::text::string` で利用可能
- StringBuilder が動作する
- fixture 8 件以上 pass
- 既存 string fixture に regression なし

## 注意点

1. `from_utf8` は Bytes 型 (#043) に依存 — Bytes が先に実装されていない場合は
   `Vec<u8>` を代用し、後で差し替える
2. `lines()` の改行判定: `\n` と `\r\n` の両方に対応
3. `slice_bytes` は UTF-8 boundary チェックを行い、不正な位置で Error を返す

## 次版への受け渡し

- text モジュールは std::bytes (043) の from_utf8 / to_utf8_bytes で接続される
- Rope (#047) は std::text の拡張として後続 issue で追加

## ドキュメント

- `docs/stdlib/text-reference.md`: string, builder, fmt の API リファレンス + 使用例

## 未解決論点

1. `normalize` (Unicode NFC/NFD) を v3 に入れるか v4 に送るか
2. `format!` マクロ的構文を v3 で入れるか
3. `Char` モジュール (is_digit, is_alpha, to_upper_char 等) の範囲
