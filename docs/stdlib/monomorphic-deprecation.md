# Monomorphic API Deprecation Table

> Monomorphic (type-suffixed) APIs are being phased out in favor of generic / trait equivalents.
> Deprecated names continue to work but will emit W0008 warnings when the resolver is wired.
> Migration follows ADR-036 D2 / ADR-014: stable APIs require ≥1 release deprecation before removal.

Columns:

| Column | Meaning |
|--------|---------|
| Stability | Current (or intended) manifest stability |
| Replacement | Preferred API |
| Deprecated since | Release that started W0008 (TBD until C1) |
| Remove in | Earliest release that may delete the entry |
| Status | Tracking state |

## Vec Constructors

| API | Stability | Replacement | Deprecated since | Remove in | Status |
|-----|-----------|-------------|------------------|-----------|--------|
| `Vec_new_i32()` | stable | `Vec::new<i32>()` | TBD | TBD | deprecated_by in manifest |
| `Vec_new_i64()` | stable | `Vec::new<i64>()` | TBD | TBD | deprecated_by in manifest |
| `Vec_new_f64()` | stable | `Vec::new<f64>()` | TBD | TBD | planned |
| `Vec_new_String()` | stable | `Vec::new<String>()` | TBD | TBD | planned |
| `Vec_new_i32_with_cap(n)` | stable | `Vec::with_capacity<i32>(n)` | TBD | TBD | planned |
| `Vec_new_i64_with_cap(n)` | stable | `Vec::with_capacity<i64>(n)` | TBD | TBD | planned |
| `Vec_new_f64_with_cap(n)` | stable | `Vec::with_capacity<f64>(n)` | TBD | TBD | planned |

## Sort

| API | Stability | Replacement | Deprecated since | Remove in | Status |
|-----|-----------|-------------|------------------|-----------|--------|
| `sort_i32(v)` | stable | `v.sort()` (`Ord`) | TBD | TBD | planned |
| `sort_i64(v)` | stable | `v.sort()` (`Ord`) | TBD | TBD | planned |
| `sort_f64(v)` | stable | `v.sort_by(total_cmp)` | TBD | TBD | planned — `f64` is not `Ord` |
| `sort_String(v)` | stable | `v.sort()` (`Ord`) | TBD | TBD | planned |

## Collection Operations

| API | Stability | Replacement | Deprecated since | Remove in | Status |
|-----|-----------|-------------|------------------|-----------|--------|
| `map_i32_i32(v, f)` | stable | `v.iter().map(f)...` | TBD | TBD | planned |
| `map_i64_i64(v, f)` | stable | `v.iter().map(f)...` | TBD | TBD | planned |
| `map_f64_f64(v, f)` | stable | `v.iter().map(f)...` | TBD | TBD | planned |
| `map_String_String(v, f)` | stable | `v.iter().map(f)...` | TBD | TBD | planned |
| `filter_i32(v, f)` | stable | `v.iter().filter(f)...` | TBD | TBD | deprecated_by in manifest |
| `filter_i64(v, f)` | stable | `v.iter().filter(f)...` | TBD | TBD | planned |
| `filter_f64(v, f)` | stable | `v.iter().filter(f)...` | TBD | TBD | planned |
| `filter_String(v, f)` | stable | `v.iter().filter(f)...` | TBD | TBD | planned |
| `fold_i32_i32(v, init, f)` | stable | `v.iter().fold(init, f)` | TBD | TBD | planned |
| `fold_i64_i64(v, init, f)` | stable | `v.iter().fold(init, f)` | TBD | TBD | planned |
| `fold_f64_f64(v, init, f)` | stable | `v.iter().fold(init, f)` | TBD | TBD | planned |
| `contains_i32(v, x)` | stable | `v.contains(x)` | TBD | TBD | planned |
| `contains_String(v, x)` | stable | `v.contains(x)` | TBD | TBD | planned |
| `reverse_i32(v)` | stable | `v.reverse()` | TBD | TBD | planned |
| `reverse_String(v)` | stable | `v.reverse()` | TBD | TBD | planned |
| `remove_i32(v, idx)` | stable | `v.remove(idx)` | TBD | TBD | planned |
| `sum_i32(v)` | stable | `v.iter().sum()` | TBD | TBD | planned |
| `sum_i64(v)` | stable | `v.iter().sum()` | TBD | TBD | planned |
| `sum_f64(v)` | stable | `v.iter().sum()` | TBD | TBD | planned |
| `product_i32(v)` | stable | `v.iter().product()` | TBD | TBD | planned |
| `product_i64(v)` | stable | `v.iter().product()` | TBD | TBD | planned |
| `product_f64(v)` | stable | `v.iter().product()` | TBD | TBD | planned |
| `any_i32(v, f)` | stable | `v.iter().any(f)` | TBD | TBD | planned |
| `any_String(v, f)` | stable | `v.iter().any(f)` | TBD | TBD | planned |
| `find_i32(v, f)` | stable | `v.iter().find(f)` | TBD | TBD | planned |
| `find_String(v, f)` | stable | `v.iter().find(f)` | TBD | TBD | planned |

## HashMap Constructors

| API | Stability | Replacement | Deprecated since | Remove in | Status |
|-----|-----------|-------------|------------------|-----------|--------|
| `HashMap_new_i32_i32()` | stable | `HashMap::new<i32, i32>()` | TBD | TBD | planned |
| `HashMap_new_i32_String()` | stable | `HashMap::new<i32, String>()` | TBD | TBD | planned |
| `HashMap_new_String_i32()` | stable | `HashMap::new<String, i32>()` | TBD | TBD | planned |
| `HashMap_new_String_String()` | stable | `HashMap::new<String, String>()` | TBD | TBD | planned |
| `HashMap_i32_i32_new()` | stable | `HashMap::new<i32, i32>()` | TBD | TBD | planned |
| `HashMap_i32_i32_insert(m, k, v)` | stable | `m.insert(k, v)` | TBD | TBD | planned |
| `HashMap_i32_i32_get(m, k)` | stable | `m.get(k)` | TBD | TBD | planned |
| `HashMap_i32_i32_contains_key(m, k)` | stable | `m.contains_key(k)` | TBD | TBD | planned |
| `HashMap_i32_i32_len(m)` | stable | `m.len()` | TBD | TBD | planned |

## Option/Result Map

| API | Stability | Replacement | Deprecated since | Remove in | Status |
|-----|-----------|-------------|------------------|-----------|--------|
| `map_option_i32_i32(o, f)` | stable | `o.map(f)` | TBD | TBD | planned |
| `map_option_String_String(o, f)` | stable | `o.map(f)` | TBD | TBD | planned |
| `map_result_i32_i32(r, f)` | stable | `r.map(f)` | TBD | TBD | planned |

## Comparison trait migration

| API | Stability | Replacement | Deprecated since | Remove in | Status |
|-----|-----------|-------------|------------------|-----------|--------|
| `Eq::eq` method ownership | stable | `PartialEq::eq` (`Eq` becomes marker) | TBD | TBD | planned (ADR-036) |

## See Also

- [ADR-036](../adr/ADR-036-trait-stdlib-redesign.md)
- [trait-stdlib-redesign.md](trait-stdlib-redesign.md)
- [ADR-014](../adr/ADR-014-stability-labels.md)
