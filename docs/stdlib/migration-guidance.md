# Deprecated API Migration Guidance

> This guide documents manifest-backed deprecations and separately records
> planned trait-first migrations. A replacement shown in a planned table is
> not necessarily callable yet. For the authoritative lifecycle table, see
> [monomorphic-deprecation.md](monomorphic-deprecation.md).

## Why APIs Are Being Deprecated

Arukellt's stdlib still contains legacy free functions and monomorphic
helpers. The trait-first v2 design is being introduced incrementally: a
replacement is documented as available only after its manifest entry,
implementation, and both-target fixture evidence exist. Roadmap examples use
the eventual method/trait shape but are explicitly marked as planned below.

Deprecated APIs continue to compile and run and emit **W0009** warnings from
the post-resolve lint pass. They will be removed in a future major
version per the [Stability Policy](stability-policy.md#deprecation-process).

---

## Currently Deprecated APIs

Only these four public entries currently carry `deprecated_by` in
`std/manifest.toml`; they are shown with ~~strikethrough~~ in the
[API reference](reference.md).

| Deprecated API | Replacement |
|----------------|-------------|
| `char_at` | `byte_at` |
| `concat` | `std::text::concat` |
| `std::env::get_var` | `std::env::var` |
| `std::host::fs::exists` | `std::host::fs::is_readable_file` |

The `std::seq` helpers and the legacy `Vec_*` names are not in this list yet.
They remain in their current stability tier until a callable replacement and a
manifest-level deprecation target are available.

## Iterator v2 status

The current iterator implementation is intentionally narrower than the final
trait-first design. The following path is implemented and verified for both
`wasm32-gc` and `wasm32`:

```ark
use std::core
use std::core::iter

fn main() {
    let values: Vec<i32> = core::range_inclusive_new(4, 6).into_iter().collect()
    let collected: Vec<i32> = Vec::from_iter(core::range_new(0, 3).into_iter())
    let count = core::range_new(0, 3).into_iter().count()
    let total = core::range_new(0, 3).into_iter().sum()
}
```

`Vec<i32>`, `Range`, and `RangeInclusive` implement `IntoIterator`; the
canonical `Iterator::collect`, `Vec::from_iter`, `count`, `sum`, and `fold`
consumers currently cover the concrete `i32` item lane. `fold` accepts a
named `fn(i32, i32) -> i32` callback on both published Wasm targets.
Structural `for item in source` lowering now crosses the same
`IntoIterator::into_iter` boundary for those concrete sources before calling
the iterator's canonical `next` method. The GC and linear-memory backends
therefore observe the same source conversion and item sequence. `values(v)`
remains an explicit compatibility form.
The source can be a direct call or method expression as well as a named local;
the range fixture verifies `for item in core::range_new(2, 5)` and
`for item in core::range_inclusive_new(3, 5).into_iter()` on both backends.
Generic or unregistered source types still require the corresponding concrete
trait implementation and are not covered by this migration step.
Statically known scalar or nominal sources that implement neither
`IntoIterator` nor `Iterator`, and do not expose the structural `next` method,
are rejected during type checking with E0200. The negative contract is pinned
by `tests/fixtures/selfhost/for_deny_non_iterable.ark` and its
`.selfhost.diag` golden; unresolved generic sources remain deferred until
their concrete obligation is available.
`FromIterator` dispatches the `VecIter`, `RangeIter`, and
`RangeInclusiveIter` sources statically. A generic consumer can now return
`Vec<T::Item>` for the currently implemented `i32`-backed iterator types;
the compiler resolves that projection during monomorphization and emits the
same concrete aggregate representation on both Wasm targets. Adding iterator
implementations for other item representations remains planned. The current
`Vec<i32>` method surface includes `iter()`, `map`, `filter`, `take`, and
`skip`; the verified composed path
`iter().map(f).filter(g).take(n).collect()` remains lazy and uses the same
canonical `Iterator::collect` traversal. Additional adapter compositions are
tracked under [#691](../../issues/open/691-iterator-trait-lazy-adapters.md)
and [#860](../../issues/open/860-iterator-unification-integration.md). The
`std::seq` helpers must not be marked deprecated until the method-level
manifest contract exists.
The concrete composed adapter is also a valid direct `for` source in this lane;
`iterator_adapters.ark` checks the `map/filter/take` chain through its identity
`IntoIterator::into_iter` conversion and canonical `next` path on both GC and
linear-memory Wasm. This remains a concrete fixture contract, not a claim that
arbitrary adapter types or item representations have completed migration.

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

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v = Vec_new_f64()
let buf = Vec_new_i32_with_cap(64)
```

**After:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
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

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
sort_i32(numbers)
sort_String(names)
```

**After:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
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

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let doubled = map_i32_i32(numbers, fn(x: i32) -> i32 { x * 2 })
let total = fold_i32_i32(numbers, 0, fn(acc: i32, x: i32) -> i32 { acc + x })
let has_it = contains_i32(numbers, 42)
reverse_i32(numbers)
```

**After:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
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
| `HashMap_i32_i32_insert(m, k, v)` | `m.insert(k, v)` |
| `HashMap_i32_i32_get(m, k)` | `m.get(k)` |
| `HashMap_i32_i32_contains_key(m, k)` | `m.contains_key(k)` |
| `HashMap_i32_i32_remove(m, k)` | `m.remove(k)` |
| `HashMap_i32_i32_len(m)` | `m.len()` |
| `HashMap_i32_i32_is_empty(m)` | `m.is_empty()` |

**Before:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let m = HashMap_new_i32_i32()
HashMap_i32_i32_insert(m, 1, 100)
let v = HashMap_i32_i32_get(m, 1)
```

**After:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let m = HashMap::new<i32, i32>()
let _ = m.insert(1, 100)
let v = m.get(1)
```

The generic method returns `Option<V>` for `insert`, `get`, and `remove`; use a pattern
match when the previous value must be inspected. `contains_key`, `len`, and `is_empty`
are direct receiver methods. The currently supported concrete pairs and the removal
conditions for the compatibility names are fixed in [ADR-066](../adr/ADR-066-language-v2-generic-public-api.md).

The old HashMap names remain callable during the ADR-014 compatibility window. They are
not marked with manifest `deprecated_by` until the manifest can name a method-level
replacement; adding an unverifiable replacement string would make the deprecation record
misleading.

### Option/Result Map

| Old API | Replacement |
|---------|-------------|
| `map_option_i32_i32(o, f)` | `map_option<i32, i32>(o, f)` |
| `map_option_String_String(o, f)` | `map_option<String, String>(o, f)` |
| `map_result_i32_i32(r, f)` | `map_result<i32, i32>(r, f)` |

**Before:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let doubled = map_option_i32_i32(maybe_val, fn(x: i32) -> i32 { x * 2 })
```

**After:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
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

4. **Check warnings**: Deprecation warnings (W0009) flag remaining deprecated
   calls and print the manifest-recorded replacement. Verify availability in
   the generated reference before migrating; a replacement string alone is not
   compile evidence.

## Deprecation Timeline

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1 — Mark | **Active** | `deprecated_by` added to manifest for initial batch (`Vec_new_i32`, `Vec_new_i64`, `filter_i32`) |
| Phase 2 — Expand | Planned | Remaining monomorphic APIs get `deprecated_by` as generic implementations land |
| Phase 3 — Warn | **Active** | Post-resolve lint emits W0009 for deprecated API usage |
| Phase 4 — Remove | Planned | Deprecated APIs removed in next major version |

## See Also

- [API Reference](reference.md) — deprecated entries shown with ~~strikethrough~~
- [Monomorphic Deprecation Table](monomorphic-deprecation.md) — concise status table
- [Stability Policy](stability-policy.md) — deprecation lifecycle rules
- [Prelude Migration (v3)](prelude-migration.md) — historical migration context
- [Error Code W0009](../compiler/error-codes.md#w0009-deprecated-api) — deprecation warning
