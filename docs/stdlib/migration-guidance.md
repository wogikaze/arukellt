# Deprecated API Migration Guidance

> This guide documents deprecated stdlib APIs, their generic replacements, and
> migration examples. For the full deprecation table, see
> [monomorphic-deprecation.md](monomorphic-deprecation.md).

## Why APIs Are Being Deprecated

Arukellt's stdlib originally provided **monomorphic** (type-suffixed) functions
such as `Vec_new_i32()`, `sort_i32()`, and `filter_i32()`. These have been
superseded by **generic** equivalents (`Vec::new<i32>()`, `sort<i32>()`,
`filter<i32>()`) that reduce API surface while preserving identical runtime
behavior.

Deprecated APIs continue to compile and run, but will emit **W0008** warnings
once the deprecation resolver is wired. They will be removed in a future major
version per the [Stability Policy](stability-policy.md#deprecation-process).

---

## Currently Deprecated APIs

These functions already carry `deprecated_by` in `std/manifest.toml` and are
shown with ~~strikethrough~~ in the [API reference](reference.md).

### `Vec_new_i32` → `Vec::new<i32>`

Creates an empty `Vec<i32>`.

**Before:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v = Vec_new_i32()
push(v, 1)
push(v, 2)
```

**After:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v = Vec::new<i32>()
push(v, 1)
push(v, 2)
```

### `Vec_new_i64` → `Vec::new<i64>`

Creates an empty `Vec<i64>`.

**Before:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v = Vec_new_i64()
push(v, 100i64)
```

**After:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v = Vec::new<i64>()
push(v, 100i64)
```

### `filter_i32` → `filter<i32>`

Filters a `Vec<i32>` by a predicate.

**Before:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let evens = filter_i32(numbers, fn(x: i32) -> bool { x % 2 == 0 })
```

**After:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let evens = filter<i32>(numbers, fn(x: i32) -> bool { x % 2 == 0 })
```

---

## Planned Deprecations

The following APIs are scheduled for deprecation via the same monomorphic →
generic pattern. They still work without warnings today but will follow the same
lifecycle once their generic counterparts are wired.

### Vec Constructors

| Old API | Replacement |
|---------|-------------|
| `Vec_new_f64()` | `Vec::new<f64>()` |
| `Vec_new_String()` | `Vec::new<String>()` |
| `Vec_new_i32_with_cap(n)` | `Vec::with_capacity<i32>(n)` |
| `Vec_new_i64_with_cap(n)` | `Vec::with_capacity<i64>(n)` |
| `Vec_new_f64_with_cap(n)` | `Vec::with_capacity<f64>(n)` |

**Before:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v = Vec_new_f64()
let buf = Vec_new_i32_with_cap(64)
```

**After:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v = Vec::new<f64>()
let buf = Vec::with_capacity<i32>(64)
```

### Sort

| Old API | Replacement |
|---------|-------------|
| `sort_i32(v)` | `sort<i32>(v)` |
| `sort_i64(v)` | `sort<i64>(v)` |
| `sort_f64(v)` | `sort<f64>(v)` |
| `sort_String(v)` | `sort<String>(v)` |

**Before:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
sort_i32(numbers)
sort_String(names)
```

**After:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
sort<i32>(numbers)
sort<String>(names)
```

### Collection Operations (map, filter, fold, etc.)

| Old API | Replacement |
|---------|-------------|
| `map_i32_i32(v, f)` | `map<i32, i32>(v, f)` |
| `map_i64_i64(v, f)` | `map<i64, i64>(v, f)` |
| `map_f64_f64(v, f)` | `map<f64, f64>(v, f)` |
| `map_String_String(v, f)` | `map<String, String>(v, f)` |
| `filter_i64(v, f)` | `filter<i64>(v, f)` |
| `filter_f64(v, f)` | `filter<f64>(v, f)` |
| `filter_String(v, f)` | `filter<String>(v, f)` |
| `fold_i32_i32(v, init, f)` | `fold<i32, i32>(v, init, f)` |
| `fold_i64_i64(v, init, f)` | `fold<i64, i64>(v, init, f)` |
| `fold_f64_f64(v, init, f)` | `fold<f64, f64>(v, init, f)` |
| `contains_i32(v, x)` | `contains<i32>(v, x)` |
| `contains_String(v, x)` | `contains<String>(v, x)` |
| `reverse_i32(v)` | `reverse<i32>(v)` |
| `reverse_String(v)` | `reverse<String>(v)` |
| `remove_i32(v, idx)` | `remove<i32>(v, idx)` |
| `sum_i32(v)` | `sum<i32>(v)` |
| `sum_i64(v)` | `sum<i64>(v)` |
| `sum_f64(v)` | `sum<f64>(v)` |
| `product_i32(v)` | `product<i32>(v)` |
| `product_i64(v)` | `product<i64>(v)` |
| `product_f64(v)` | `product<f64>(v)` |
| `any_i32(v, f)` | `any<i32>(v, f)` |
| `any_String(v, f)` | `any<String>(v, f)` |
| `find_i32(v, f)` | `find<i32>(v, f)` |
| `find_String(v, f)` | `find<String>(v, f)` |

**Before:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let doubled = map_i32_i32(numbers, fn(x: i32) -> i32 { x * 2 })
let total = fold_i32_i32(numbers, 0, fn(acc: i32, x: i32) -> i32 { acc + x })
let has_it = contains_i32(numbers, 42)
reverse_i32(numbers)
```

**After:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let doubled = map<i32, i32>(numbers, fn(x: i32) -> i32 { x * 2 })
let total = fold<i32, i32>(numbers, 0, fn(acc: i32, x: i32) -> i32 { acc + x })
let has_it = contains<i32>(numbers, 42)
reverse<i32>(numbers)
```

### HashMap Constructors and Operations

| Old API | Replacement |
|---------|-------------|
| `HashMap_new_i32_i32()` | `HashMap::new<i32, i32>()` |
| `HashMap_new_i32_String()` | `HashMap::new<i32, String>()` |
| `HashMap_new_String_i32()` | `HashMap::new<String, i32>()` |
| `HashMap_new_String_String()` | `HashMap::new<String, String>()` |
| `HashMap_i32_i32_new()` | `HashMap::new<i32, i32>()` |
| `HashMap_i32_i32_insert(m, k, v)` | `insert(m, k, v)` |
| `HashMap_i32_i32_get(m, k)` | `get(m, k)` |
| `HashMap_i32_i32_contains_key(m, k)` | `contains_key(m, k)` |
| `HashMap_i32_i32_len(m)` | `len(m)` |

**Before:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let m = HashMap_new_i32_i32()
HashMap_i32_i32_insert(m, 1, 100)
let v = HashMap_i32_i32_get(m, 1)
```

**After:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let m = HashMap::new<i32, i32>()
insert(m, 1, 100)
let v = get(m, 1)
```

### Option/Result Map

| Old API | Replacement |
|---------|-------------|
| `map_option_i32_i32(o, f)` | `map_option<i32, i32>(o, f)` |
| `map_option_String_String(o, f)` | `map_option<String, String>(o, f)` |
| `map_result_i32_i32(r, f)` | `map_result<i32, i32>(r, f)` |

**Before:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let doubled = map_option_i32_i32(maybe_val, fn(x: i32) -> i32 { x * 2 })
```

**After:**

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let doubled = map_option<i32, i32>(maybe_val, fn(x: i32) -> i32 { x * 2 })
```

---

## Migration Steps

1. **Find deprecated calls**: Search your codebase for monomorphic function names
   (any function with a type suffix like `_i32`, `_i64`, `_f64`, `_String`).

2. **Replace with generic equivalent**: Swap the type suffix for angle-bracket
   generic syntax. The function signature is otherwise identical.

3. **Test**: Deprecated and generic versions have identical runtime behavior.
   Swapping one for the other should not change program output.

4. **Check warnings**: Once deprecation warnings (W0008) are active, the
   compiler will flag any remaining deprecated calls.

## Deprecation Timeline

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1 — Mark | **Active** | `deprecated_by` added to manifest for initial batch (`Vec_new_i32`, `Vec_new_i64`, `filter_i32`) |
| Phase 2 — Expand | Planned | Remaining monomorphic APIs get `deprecated_by` as generic implementations land |
| Phase 3 — Warn | Planned | Resolver emits W0008 for deprecated API usage |
| Phase 4 — Remove | Planned | Deprecated APIs removed in next major version |

## See Also

- [API Reference](reference.md) — deprecated entries shown with ~~strikethrough~~
- [Monomorphic Deprecation Table](monomorphic-deprecation.md) — concise status table
- [Stability Policy](stability-policy.md) — deprecation lifecycle rules
- [Prelude Migration (v3)](prelude-migration.md) — historical migration context
- [Error Code W0008](../compiler/error-codes.md#w0008--deprecated-api) — deprecation warning
