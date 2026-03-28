# Arukellt 標準ライブラリ

Arukellt stdlib は **"tiny prelude, large explicit stdlib"** を設計原則とする。  
prelude に自動 import されるものは最小限にし、機能は明示 import で到達する。

> **実装状態の基準**:
> - 現行実装 → [current-state.md](../current-state.md) と `std/prelude.ark`
> - v3 設計方針 → [std.md](std.md) (多言語調査を踏まえた総合設計書)
> - v3 完了条件 → [roadmap-v3.md](../process/roadmap-v3.md)

---

## モジュール構成

| モジュール | 状態 | 説明 |
|-----------|------|------|
| `std::prelude` | ✅ Stable | `Option`, `Result`, `String`, `Vec`, `println`, `assert` 等の自動 import |
| `std::core` | 🔨 v3 | `Ordering`, `Range`, `Error`, math, cmp, hash, panic |
| `std::text` | 🔨 v3 | `String`, char, builder, fmt, pattern |
| `std::bytes` | 🔨 v3 | `Bytes`, `ByteBuf`, `ByteCursor`, endian, hex, base64, leb128 |
| `std::collections` | 🔨 v3 | `Vec`, `HashMap`, `HashSet`, `Deque`, `BTreeMap`, `IndexMap`, `BitSet` |
| `std::seq` | 🔨 v3 | `Seq<T>` 遅延パイプライン: map/filter/fold/collect |
| `std::io` | 🔨 v3 | Reader, Writer, stdin/out/err |
| `std::fs` | 🔶 partial | `fs_read_file`, `fs_write_file` (WASI P1/P2) |
| `std::path` | 🔨 v3 | join, parent, file_name, extension, normalize |
| `std::process` | 🔨 v3 | `args`, `exit`, `env_var` |
| `std::time` | 🔶 partial | `clock_now() -> i64` |
| `std::random` | 🔶 partial | `random_i32()` |
| `std::json` | 🔮 v4 | JSON parse/stringify |
| `std::wasm` | 🔮 v3/v4 | Wasm binary builder, leb128, val types |
| `std::wit` | 🔮 v4 | WIT 型・world・interface |
| `std::component` | 🔮 v4 | `Own<T>`, `Borrow<T>`, canonical ABI adapter |
| `std::test` | 🔨 v3 | `assert_eq`, `assert_ne`, `expect_err`, snapshot |

**凡例**: ✅ 実装済み安定 / 🔶 部分実装 / 🔨 v3 で実装予定 / 🔮 v4 以降

---

## prelude で自動 import されるもの

```ark
// 型
Option<T>  Result<T, E>  String  Vec<T>

// コンストラクタ
Some(x)  None  Ok(x)  Err(e)

// 出力
println(s)  print(s)  eprintln(s)

// アサーション
assert(cond)  assert_eq(a, b)  assert_ne(a, b)

// panic
panic(msg)
```

全ての実装済み関数は [modules/core.md](modules/core.md) 以降に記載。

---

## 現行 API (v2 互換)

v3 移行前の現行 prelude API 一覧:

```ark
// 文字列
String_from(s)  String_new()  eq(a,b)  concat(a,b)  clone(s)  slice(s,i,j)
split(s,sep)  join(v,sep)  starts_with(s,prefix)  ends_with(s,suffix)
to_lower(s)  to_upper(s)  push_char(s,c)  trim(s)  contains(s,needle)
replace(s,from,to)  string_len(s)  char_at(s,i)  index_of(s,needle)

// 型変換
i32_to_string(n)  i64_to_string(n)  f64_to_string(f)  bool_to_string(b)
char_to_string(c)  parse_i32(s)  parse_i64(s)  parse_f64(s)

// 数学
sqrt(f)  abs(n)  min(a,b)  max(a,b)  clamp_i32(v,lo,hi)

// Vec (モノモーフ名 — v3 で deprecated 予定)
Vec_new_i32()  Vec_new_i64()  Vec_new_f64()  Vec_new_String()
push(v,x)  pop(v)  get(v,i)  set(v,i,x)  len(v)
sort_i32(v)  sort_i64(v)  sort_f64(v)  sort_String(v)
contains_i32(v,x)  contains_String(v,x)
reverse_i32(v)  reverse_String(v)
remove_i32(v,i)  sum_i32(v)  product_i32(v)

// Vec HOF
map_i32_i32(v,f)  filter_i32(v,f)  fold_i32_i32(v,init,f)
map_i64_i64(v,f)  filter_i64(v,f)  fold_i64_i64(v,init,f)
map_f64_f64(v,f)  filter_f64(v,f)
map_String_String(v,f)  filter_String(v,f)
any_i32(v,f)  find_i32(v,f)
map_option_i32_i32(opt,f)

// I/O (WASI)
fs_read_file(path)  fs_write_file(path,content)
clock_now()  random_i32()

// HashMap (v2 追加)
HashMap_new_String_i32()  HashMap_new_i32_i32()  HashMap_new_String_String()
hashmap_insert(m,k,v)  hashmap_get(m,k)  hashmap_contains_key(m,k)
hashmap_remove(m,k)  hashmap_len(m)  hashmap_keys(m)  hashmap_values(m)
```

v3 での移行先は [modules/collections.md](modules/collections.md) と migration guide を参照。

---

## ドキュメント構成

| ファイル | 内容 |
|---------|------|
| [std.md](std.md) | 多言語調査を踏まえた総合設計書 (v3 設計の根拠) |
| [reference.md](reference.md) | 全 API + 安定性ラベル (生成物) |
| [modules/core.md](modules/core.md) | Option / Result / math / cmp / panic |
| [modules/text.md](modules/text.md) | String / char / fmt |
| [modules/bytes.md](modules/bytes.md) | Bytes / ByteBuf / leb128 / endian |
| [modules/collections.md](modules/collections.md) | Vec / HashMap / HashSet / Deque |
| [modules/seq.md](modules/seq.md) | Seq<T> 遅延パイプライン |
| [modules/io.md](modules/io.md) | fs / path / process / io |
| [modules/wasm.md](modules/wasm.md) | std::wasm / std::wit / std::component |
| [modules/test.md](modules/test.md) | assert / snapshot / bench |
| [cookbook.md](cookbook.md) | タスク別使用例 |
