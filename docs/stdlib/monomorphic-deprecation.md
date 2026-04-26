# Monomorphic API Deprecation Table

> Monomorphic (type-suffixed) APIs are being phased out in favor of generic equivalents.
> Deprecated names continue to work but will emit W0008 warnings when the resolver is wired.

## Vec Constructors

| Deprecated | Replacement | Status |
|-----------|------------|--------|
| `Vec_new_i32()` | `Vec::new<i32>()` | deprecated_by in manifest |
| `Vec_new_i64()` | `Vec::new<i64>()` | deprecated_by in manifest |
| `Vec_new_f64()` | `Vec::new<f64>()` | planned |
| `Vec_new_String()` | `Vec::new<String>()` | planned |
| `Vec_new_i32_with_cap(n)` | `Vec::with_capacity<i32>(n)` | planned |
| `Vec_new_i64_with_cap(n)` | `Vec::with_capacity<i64>(n)` | planned |
| `Vec_new_f64_with_cap(n)` | `Vec::with_capacity<f64>(n)` | planned |

## Sort

| Deprecated | Replacement | Status |
|-----------|------------|--------|
| `sort_i32(v)` | `sort<i32>(v)` | planned |
| `sort_i64(v)` | `sort<i64>(v)` | planned |
| `sort_f64(v)` | `sort<f64>(v)` | planned |
| `sort_String(v)` | `sort<String>(v)` | planned |

## Collection Operations

| Deprecated | Replacement | Status |
|-----------|------------|--------|
| `map_i32_i32(v, f)` | `map<i32, i32>(v, f)` | planned |
| `map_i64_i64(v, f)` | `map<i64, i64>(v, f)` | planned |
| `map_f64_f64(v, f)` | `map<f64, f64>(v, f)` | planned |
| `map_String_String(v, f)` | `map<String, String>(v, f)` | planned |
| `filter_i32(v, f)` | `filter<i32>(v, f)` | deprecated_by in manifest |
| `filter_i64(v, f)` | `filter<i64>(v, f)` | planned |
| `filter_f64(v, f)` | `filter<f64>(v, f)` | planned |
| `filter_String(v, f)` | `filter<String>(v, f)` | planned |
| `fold_i32_i32(v, init, f)` | `fold<i32, i32>(v, init, f)` | planned |
| `fold_i64_i64(v, init, f)` | `fold<i64, i64>(v, init, f)` | planned |
| `fold_f64_f64(v, init, f)` | `fold<f64, f64>(v, init, f)` | planned |
| `contains_i32(v, x)` | `contains<i32>(v, x)` | planned |
| `contains_String(v, x)` | `contains<String>(v, x)` | planned |
| `reverse_i32(v)` | `reverse<i32>(v)` | planned |
| `reverse_String(v)` | `reverse<String>(v)` | planned |
| `remove_i32(v, idx)` | `remove<i32>(v, idx)` | planned |
| `sum_i32(v)` | `sum<i32>(v)` | planned |
| `sum_i64(v)` | `sum<i64>(v)` | planned |
| `sum_f64(v)` | `sum<f64>(v)` | planned |
| `product_i32(v)` | `product<i32>(v)` | planned |
| `product_i64(v)` | `product<i64>(v)` | planned |
| `product_f64(v)` | `product<f64>(v)` | planned |
| `any_i32(v, f)` | `any<i32>(v, f)` | planned |
| `any_String(v, f)` | `any<String>(v, f)` | planned |
| `find_i32(v, f)` | `find<i32>(v, f)` | planned |
| `find_String(v, f)` | `find<String>(v, f)` | planned |

## HashMap Constructors

| Deprecated | Replacement | Status |
|-----------|------------|--------|
| `HashMap_new_i32_i32()` | `HashMap::new<i32, i32>()` | planned |
| `HashMap_new_i32_String()` | `HashMap::new<i32, String>()` | planned |
| `HashMap_new_String_i32()` | `HashMap::new<String, i32>()` | planned |
| `HashMap_new_String_String()` | `HashMap::new<String, String>()` | planned |
| `HashMap_i32_i32_new()` | `HashMap::new<i32, i32>()` | planned |
| `HashMap_i32_i32_insert(m, k, v)` | `insert(m, k, v)` | planned |
| `HashMap_i32_i32_get(m, k)` | `get(m, k)` | planned |
| `HashMap_i32_i32_contains_key(m, k)` | `contains_key(m, k)` | planned |
| `HashMap_i32_i32_len(m)` | `len(m)` | planned |

## Option/Result Map

| Deprecated | Replacement | Status |
|-----------|------------|--------|
| `map_option_i32_i32(o, f)` | `map_option<i32, i32>(o, f)` | planned |
| `map_option_String_String(o, f)` | `map_option<String, String>(o, f)` | planned |
| `map_result_i32_i32(r, f)` | `map_result<i32, i32>(r, f)` | planned |

## See Also

- [Stability Policy](stability-policy.md)
- [Prelude Migration](prelude-migration.md)
- [Error Code W0008](../compiler/error-codes.md#w0008--deprecated-api)
