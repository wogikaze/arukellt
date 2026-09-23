# Deprecated API Migration Table

> Generated from `std/manifest.toml` by `scripts/gen/generate-docs.py`.
> Lifecycle state and replacement are never maintained separately here.
> A replacement string is not proof of current callability; migration evidence remains required.

Deprecated APIs remain callable for the policy window in
[stability-policy.md](stability-policy.md). Monomorphic compatibility
helpers are included alongside any other deprecated public entry.

| API | Module | Stability | Replacement | Deprecated since | Earliest removal | Reason |
|-----|--------|-----------|-------------|------------------|------------------|--------|
| `char_at` | `prelude` | `deprecated` | `byte_at` | `0.1.0` | `1.0.0` | Return the raw byte at a byte index. Use byte_at for the explicit compatibility API. |
| `concat` | `prelude` | `deprecated` | `std::text::concat` | `0.1.0` | `1.0.0` | Concatenate two strings and return the result. |
| `get_var` | `std::env` | `deprecated` | `var` | `0.1.0` | `1.0.0` | Alias for env::var. Use var instead. |
| `exists` | `std::host::fs` | `deprecated` | `is_readable_file` | `0.1.0` | `1.0.0` | Deprecated alias for is_readable_file. Same read-probe semantics — NOT a general path-existence query. |
| `binary_search` | `std::seq` | `deprecated` | `std::seq::binary_search_by` | `0.1.0` | `1.0.0` | Deprecated monomorphic wrapper. Use the generic std::seq::binary_search_by(v, target) over T: Ord. |
| `count_eq` | `std::seq` | `deprecated` | `std::core::iter::iter_count_eq_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorCountEq capability v.iter().count_eq(target) or std::core::iter::iter_count_eq_i32(v, target). |
| `filter_i32` | `std::seq` | `deprecated` | `std::core::iter::collect_filter_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().filter(f).collect() or std::core::iter::collect_filter_i32(v, f). |
| `fold_i32_i32` | `std::seq` | `deprecated` | `std::core::iter::iter_fold_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorFold capability v.iter().fold(init, f) or std::core::iter::iter_fold_i32(v, init, f). |
| `map_i32_i32` | `std::seq` | `deprecated` | `std::core::iter::collect_map_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().map(f).collect() or std::core::iter::collect_map_i32(v, f). |
| `max_i32` | `std::seq` | `deprecated` | `std::seq::max_by` | `0.1.0` | `1.0.0` | Deprecated monomorphic wrapper. Use the generic std::seq::max_by(v) over T: Ord. |
| `min_i32` | `std::seq` | `deprecated` | `std::seq::min_by` | `0.1.0` | `1.0.0` | Deprecated monomorphic wrapper. Use the generic std::seq::min_by(v) over T: Ord. |
| `seq_contains` | `std::seq` | `deprecated` | `std::core::iter::iter_contains_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorContains capability v.iter().contains(target) or std::core::iter::iter_contains_i32(v, target). |
| `seq_reverse` | `std::seq` | `deprecated` | `std::core::iter::iter_reverse_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().rev().collect() or std::core::iter::iter_reverse_i32(v). |
| `skip_i32` | `std::seq` | `deprecated` | `std::core::iter::collect_skip_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().skip(n).collect() or std::core::iter::collect_skip_i32(v, n). |
| `sum_i32` | `std::seq` | `deprecated` | `std::core::iter::iter_sum_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorSum capability v.iter().sum() or std::core::iter::iter_sum_i32(v). |
| `take_i32` | `std::seq` | `deprecated` | `std::core::iter::collect_take_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().take(n).collect() or std::core::iter::collect_take_i32(v, n). |
| `unique` | `std::seq` | `deprecated` | `std::core::iter::iter_unique_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().unique().collect() or std::core::iter::iter_unique_i32(v). |

Total deprecated public entries: **17**.
