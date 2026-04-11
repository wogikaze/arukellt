# std::json + std::toml + std::csv: データ形式パーサ

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 055
**Depends on**: 039, 042, 044
**Track**: stdlib
**Blocks v3 exit**: no (Experimental — json のみ Stable 候補)

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/055-std-json-toml-csv.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

実務データ形式 (JSON, TOML, CSV) のパーサとシリアライザを実装する。
設定ファイル読み込み、データ交換、CLI ツール出力に使用。
JSON は Stable 候補、TOML/CSV は Experimental。

## 受け入れ条件

### std::json

```ark
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),  // IndexMap 的に insertion order 保持
}

pub fn parse(s: String) -> Result<JsonValue, Error>
pub fn stringify(v: JsonValue) -> String
pub fn stringify_pretty(v: JsonValue, indent: i32) -> String

pub fn json_get(v: JsonValue, key: String) -> Option<JsonValue>
pub fn json_get_index(v: JsonValue, index: i32) -> Option<JsonValue>
pub fn json_as_string(v: JsonValue) -> Option<String>
pub fn json_as_i32(v: JsonValue) -> Option<i32>
pub fn json_as_f64(v: JsonValue) -> Option<f64>
pub fn json_as_bool(v: JsonValue) -> Option<bool>
pub fn json_as_array(v: JsonValue) -> Option<Vec<JsonValue>>
```

### std::toml

```ark
pub enum TomlValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<TomlValue>),
    Table(Vec<(String, TomlValue)>),
}

pub fn toml_parse(s: String) -> Result<TomlValue, Error>
pub fn toml_stringify(v: TomlValue) -> String
```

### std::csv

```ark
pub fn csv_parse(s: String) -> Result<Vec<Vec<String>>, Error>
pub fn csv_stringify(rows: Vec<Vec<String>>) -> String
pub fn csv_parse_with_header(s: String) -> Result<(Vec<String>, Vec<Vec<String>>), Error>
```

## 実装タスク

1. `std/json/json.ark`: JSON パーサ (recursive descent, source 実装)
2. `std/json/stringify.ark`: JSON シリアライザ (source 実装)
3. `std/toml/toml.ark`: TOML パーサ (basic tables + key-value, source 実装)
4. `std/csv/csv.ark`: CSV パーサ (RFC 4180 準拠, source 実装)
5. JsonValue/TomlValue 型の登録
6. json_get 等のヘルパー関数

## 検証方法

- fixture: `stdlib_json/json_parse.ark`, `stdlib_json/json_stringify.ark`,
  `stdlib_json/json_nested.ark`, `stdlib_json/json_escape.ark`,
  `stdlib_toml/toml_basic.ark`, `stdlib_csv/csv_basic.ark`,
  `stdlib_json/json_pretty.ark`

## 完了条件

- JSON parse/stringify が RFC 8259 の基本ケースで正しく動作する
- TOML が基本的な table + key-value を parse できる
- CSV が RFC 4180 のクォート付きフィールドを処理できる
- fixture 7 件以上 pass

## 注意点

1. JSON パーサの数値精度: f64 で表現。i64 を超える整数は精度損失 — 警告を出すか
2. TOML の datetime 型は v3 では String として扱い、std::time 連携は v4
3. CSV の巨大ファイルはストリーミング対応が望ましいが、v3 では全体読み込みで可

## ドキュメント

- `docs/stdlib/json-reference.md`, `docs/stdlib/toml-reference.md`, `docs/stdlib/csv-reference.md`

## 未解決論点

1. JSON の streaming parse を v3 に入れるか
2. YAML サポートを v3 スコープに含めるか (含めない推奨)
