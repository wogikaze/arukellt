# stdlib API リファレンス

**安定性ラベル**: Stable ✅ / Experimental 🔶 / v3-planned 🔨 / v4+ 🔮 / Internal 🔒

> このファイルは全 public API の安定性一覧。詳細は各モジュールページを参照。

---

## std::core

| 関数 / 型 | シグネチャ | 安定性 | 追加バージョン |
|---------|----------|--------|-------------|
| `Option<T>` | 型 | ✅ Stable | v1 |
| `Result<T, E>` | 型 | ✅ Stable | v1 |
| `Some(x)` | `T -> Option<T>` | ✅ Stable | v1 |
| `None` | `Option<T>` | ✅ Stable | v1 |
| `Ok(x)` | `T -> Result<T, E>` | ✅ Stable | v1 |
| `Err(e)` | `E -> Result<T, E>` | ✅ Stable | v1 |
| `is_some` | `Option<T> -> bool` | ✅ Stable | v1 |
| `is_none` | `Option<T> -> bool` | ✅ Stable | v1 |
| `is_ok` | `Result<T,E> -> bool` | ✅ Stable | v2 |
| `is_err` | `Result<T,E> -> bool` | ✅ Stable | v2 |
| `unwrap` | `Option<T> -> T` | ✅ Stable | v1 |
| `unwrap_or` | `Option<T>, T -> T` | ✅ Stable | v1 |
| `unwrap_or_else` | `Option<T>, fn()->T -> T` | ✅ Stable | v2 |
| `expect` | `Result<T,E>, String -> T` | ✅ Stable | v2 |
| `ok_or` | `Option<T>, E -> Result<T,E>` | ✅ Stable | v2 |
| `ok` | `Result<T,E> -> Option<T>` | ✅ Stable | v2 |
| `panic` | `String -> Never` | ✅ Stable | v1 |
| `assert` | `bool -> ()` | ✅ Stable | v1 |
| `assert_eq` | `T, T -> ()` | ✅ Stable | v1 |
| `assert_ne` | `T, T -> ()` | ✅ Stable | v1 |
| `abs` | `i32 -> i32` | ✅ Stable | v1 |
| `min` | `i32, i32 -> i32` | ✅ Stable | v1 |
| `max` | `i32, i32 -> i32` | ✅ Stable | v1 |
| `clamp_i32` | `i32, i32, i32 -> i32` | ✅ Stable | v1 |
| `sqrt` | `f64 -> f64` | ✅ Stable | v1 |
| `floor` | `f64 -> f64` | 🔨 v3 | — |
| `ceil` | `f64 -> f64` | 🔨 v3 | — |
| `round` | `f64 -> f64` | 🔨 v3 | — |
| `pow_i32` | `i32, i32 -> i32` | 🔨 v3 | — |

## 型変換

| 関数 | シグネチャ | 安定性 |
|------|----------|--------|
| `i32_to_string` | `i32 -> String` | ✅ Stable |
| `i64_to_string` | `i64 -> String` | ✅ Stable |
| `f64_to_string` | `f64 -> String` | ✅ Stable |
| `bool_to_string` | `bool -> String` | ✅ Stable |
| `char_to_string` | `char -> String` | ✅ Stable |
| `parse_i32` | `String -> Result<i32, String>` | ✅ Stable |
| `parse_i64` | `String -> Result<i64, String>` | ✅ Stable |
| `parse_f64` | `String -> Result<f64, String>` | ✅ Stable |
| `parse_bool` | `String -> Result<bool, String>` | 🔨 v3 | — |

---

## std::text

| 関数 | シグネチャ | 安定性 |
|------|----------|--------|
| `String_new` | `-> String` | ✅ Stable |
| `String_from` | `String -> String` | ✅ Stable |
| `string_len` | `String -> i32` | ✅ Stable |
| `string_len_chars` | `String -> i32` | 🔨 v3 |
| `is_empty` | `String -> bool` | ✅ Stable |
| `concat` | `String, String -> String` | ✅ Stable |
| `clone` | `String -> String` | ✅ Stable |
| `slice` | `String, i32, i32 -> String` | ✅ Stable |
| `to_lower` | `String -> String` | ✅ Stable |
| `to_upper` | `String -> String` | ✅ Stable |
| `trim` | `String -> String` | ✅ Stable |
| `trim_start` | `String -> String` | 🔨 v3 |
| `trim_end` | `String -> String` | 🔨 v3 |
| `contains` | `String, String -> bool` | ✅ Stable |
| `starts_with` | `String, String -> bool` | ✅ Stable |
| `ends_with` | `String, String -> bool` | ✅ Stable |
| `index_of` | `String, String -> Option<i32>` | 🔨 v3 |
| `replace` | `String, String, String -> String` | ✅ Stable |
| `split` | `String, String -> Vec<String>` | ✅ Stable |
| `split_once` | `String, String -> Option<(String,String)>` | 🔨 v3 |
| `lines` | `String -> Vec<String>` | 🔨 v3 |
| `join` | `Vec<String>, String -> String` | ✅ Stable |
| `char_at` | `String, i32 -> char` | ✅ Stable |
| `push_char` | `String, char -> ()` | ✅ Stable |
| `string_repeat` | `String, i32 -> String` | 🔨 v3 |
| `to_utf8_bytes` | `String -> Bytes` | 🔨 v3 |
| `from_utf8` | `Bytes -> Result<String, String>` | 🔨 v3 |
| `builder_new` | `-> StringBuilder` | 🔨 v3 |
| `builder_push` | `StringBuilder, String -> ()` | 🔨 v3 |
| `builder_finish` | `StringBuilder -> String` | 🔨 v3 |

---

## std::bytes (v3)

→ [modules/bytes.md](modules/bytes.md) 参照

| 型 / 関数 | 安定性 |
|---------|--------|
| `Bytes` | 🔨 v3 |
| `ByteBuf` | 🔨 v3 |
| `ByteCursor` | 🔨 v3 |
| `leb128::read_var_u32` | 🔨 v3 |
| `leb128::write_var_u32` | 🔨 v3 |
| `hex_encode` / `hex_decode` | 🔨 v3 |
| `base64_encode` / `base64_decode` | 🔨 v3 |

---

## std::collections

### Vec (旧 monomorph 名 — deprecated 予定)

| 関数 | 安定性 | 代替 (v3) |
|------|--------|---------|
| `Vec_new_i32` | ⚠️ Deprecated (v3) | `vec::new<i32>()` |
| `Vec_new_String` | ⚠️ Deprecated (v3) | `vec::new<String>()` |
| `map_i32_i32` | ⚠️ Deprecated (v3) | `vec::map(v, f)` |
| `filter_i32` | ⚠️ Deprecated (v3) | `vec::filter(v, f)` |
| `sort_i32` | ✅ Stable | — |
| `push` | ✅ Stable | — |
| `pop` | ✅ Stable | — |
| `get` | ✅ Stable | — |
| `set` | ✅ Stable | — |
| `len` | ✅ Stable | — |
| `contains_i32` | ✅ Stable | — |
| `contains_String` | ✅ Stable | — |
| `reverse_i32` | ✅ Stable | — |
| `remove_i32` | ✅ Stable | — |
| `sum_i32` | ✅ Stable | — |
| `product_i32` | ✅ Stable | — |

### HashMap

| 関数 | 安定性 |
|------|--------|
| `HashMap_new_String_i32` | ✅ Stable (v2) |
| `HashMap_new_i32_i32` | ✅ Stable (v2) |
| `HashMap_new_String_String` | ✅ Stable (v2) |
| `hashmap_insert` | ✅ Stable (v2) |
| `hashmap_get` | ✅ Stable (v2) |
| `hashmap_contains_key` | ✅ Stable (v2) |
| `hashmap_remove` | ✅ Stable (v2) |
| `hashmap_len` | ✅ Stable (v2) |
| `hashmap_keys` | ✅ Stable (v2) |
| `hashmap_values` | ✅ Stable (v2) |
| `hashmap_entries` | 🔨 v3 |
| `hashmap_get_or_insert` | 🔨 v3 |
| `hashmap_update` | 🔨 v3 |
| `hashmap_retain` | 🔨 v3 |

### HashSet (v3)

→ [modules/collections.md](modules/collections.md) 参照

| 関数 | 安定性 |
|------|--------|
| `hashset_new` | 🔨 v3 |
| `hashset_insert` | 🔨 v3 |
| `hashset_contains` | 🔨 v3 |
| `hashset_remove` | 🔨 v3 |
| `hashset_union` | 🔨 v3 |
| `hashset_intersection` | 🔨 v3 |

---

## std::seq (v3)

→ [modules/seq.md](modules/seq.md) 参照

| 関数 | 安定性 |
|------|--------|
| `seq_from_vec` | 🔨 v3 |
| `seq_range_i32` | 🔨 v3 |
| `seq_map` | 🔨 v3 |
| `seq_filter` | 🔨 v3 |
| `seq_fold` | 🔨 v3 |
| `seq_collect_vec` | 🔨 v3 |
| `seq_collect_hashmap` | 🔨 v3 |
| `group_by` | 🔨 v3 |
| `seq_zip` | 🔨 v3 |
| `seq_enumerate` | 🔨 v3 |

---

## std::fs / path / process / io (v3)

→ [modules/io.md](modules/io.md) 参照

| 関数 | 安定性 |
|------|--------|
| `fs_read_file` | ✅ Stable |
| `fs_write_file` | ✅ Stable |
| `fs_read_bytes` | 🔨 v3 |
| `fs_exists` | 🔨 v3 |
| `fs_mkdir_all` | 🔨 v3 |
| `fs_list_dir` | 🔨 v3 |
| `path_join` | 🔨 v3 |
| `path_parent` | 🔨 v3 |
| `path_file_name` | 🔨 v3 |
| `args` | 🔨 v3 |
| `exit` | 🔨 v3 |
| `env_var` | 🔨 v3 |
| `stdin_read_line` | 🔨 v3 |
| `clock_now` | ✅ Stable |
| `random_i32` | ✅ Stable |
| `random_i64` | 🔨 v3 |

---

## std::test

| 関数 | 安定性 |
|------|--------|
| `assert` | ✅ Stable |
| `assert_eq` | ✅ Stable |
| `assert_ne` | ✅ Stable |
| `assert_eq_i64` | ✅ Stable |
| `assert_eq_str` | ✅ Stable |
| `assert_true` | 🔨 v3 |
| `assert_false` | 🔨 v3 |
| `assert_eq_f64` | 🔨 v3 |
| `assert_ok` | 🔨 v3 |
| `assert_err` | 🔨 v3 |
| `assert_some` | 🔨 v3 |
| `assert_none` | 🔨 v3 |
| `assert_with_msg` | 🔨 v3 |
| `assert_snapshot` | 🔮 v3/v4 |
| `bench_run` | 🔶 Experimental |

---

## std::wasm / std::wit / std::component

→ [modules/wasm.md](modules/wasm.md) 参照

| モジュール | 安定性 |
|-----------|--------|
| `std::wasm::types` | 🔨 v3 (型定義のみ) |
| `std::wasm::binary` | 🔮 v4 |
| `std::wasm::instr` | 🔮 v4 |
| `std::wit` | 🔮 v4 |
| `std::component` | 🔮 v4 |

---

*このファイルは `scripts/verify-harness.sh` の stdlib manifest 整合チェックと連動させること (v3 で追加予定)。*
