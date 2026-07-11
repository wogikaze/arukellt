# Deprecated API Migration Table

> Generated from `std/manifest.toml` by `scripts/gen/generate-docs.py`.
> Lifecycle state and replacement are never maintained separately here.

Deprecated APIs remain callable for the policy window in
[stability-policy.md](stability-policy.md). Monomorphic compatibility
helpers are included alongside any other deprecated public entry.

| API | Module | Stability | Replacement | Deprecated since | Earliest removal | Reason |
|-----|--------|-----------|-------------|------------------|------------------|--------|
| `HashMap_String_String_contains_key` | `prelude` | `deprecated` | `hashmap_str_str_contains` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_String_String_get` | `prelude` | `deprecated` | `hashmap_str_str_get` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_String_String_insert` | `prelude` | `deprecated` | `hashmap_str_str_insert` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_String_String_len` | `prelude` | `deprecated` | `hashmap_str_str_len` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_String_i32_contains_key` | `prelude` | `deprecated` | `hashmap_str_i32_contains` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_String_i32_get` | `prelude` | `deprecated` | `hashmap_str_i32_get` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_String_i32_insert` | `prelude` | `deprecated` | `hashmap_str_i32_insert` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_String_i32_len` | `prelude` | `deprecated` | `hashmap_str_i32_len` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_i32_String_contains_key` | `prelude` | `deprecated` | `hashmap_i32_str_contains` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_i32_String_get` | `prelude` | `deprecated` | `hashmap_i32_str_get` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_i32_String_insert` | `prelude` | `deprecated` | `hashmap_i32_str_insert` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_i32_String_len` | `prelude` | `deprecated` | `hashmap_i32_str_len` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_i32_i32_contains_key` | `prelude` | `deprecated` | `std::collections::hash_map` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_i32_i32_get` | `prelude` | `deprecated` | `std::collections::hash_map` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_i32_i32_insert` | `prelude` | `deprecated` | `hashmap_i32_i32_insert` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_i32_i32_len` | `prelude` | `deprecated` | `std::collections::hash_map` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_i32_i32_new` | `prelude` | `deprecated` | `hashmap_i32_i32_new` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_new_String_String` | `prelude` | `deprecated` | `hashmap_str_str_new` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_new_String_i32` | `prelude` | `deprecated` | `hashmap_str_i32_new` | `0.1.0` | `1.0.0` | Superseded API |
| `HashMap_new_i32_String` | `prelude` | `deprecated` | `hashmap_i32_str_new` | `0.1.0` | `1.0.0` | Superseded API |
| `Vec_new_i32` | `prelude` | `deprecated` | `Vec::new<i32>` | `0.1.0` | `1.0.0` | Superseded API |
| `Vec_new_i64` | `prelude` | `deprecated` | `Vec::new<i64>` | `0.1.0` | `1.0.0` | Superseded API |
| `Vec_new_v128` | `prelude` | `deprecated` | `Vec::new<v128>` | `0.1.0` | `1.0.0` | Superseded API |
| `concat` | `prelude` | `deprecated` | `std::text::concat` | `0.1.0` | `1.0.0` | Concatenate two strings and return the result. |
| `filter_i32` | `prelude` | `deprecated` | `filter<i32>` | `0.1.0` | `1.0.0` | Superseded API |
| `get_var` | `std::env` | `deprecated` | `var` | `0.1.0` | `1.0.0` | Alias for env::var. Use var instead. |
| `exists` | `std::host::fs` | `deprecated` | `is_readable_file` | `0.1.0` | `1.0.0` | Deprecated alias for is_readable_file. Same read-probe semantics — NOT a general path-existence query. |

Total deprecated public entries: **27**.
