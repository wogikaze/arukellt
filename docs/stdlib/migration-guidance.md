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

Deprecated module-qualified APIs continue to compile and run and emit
**W0009** warnings from the post-resolve lint pass. Deprecated bare prelude
bindings (`char_at`, `concat`, `print`, `println`) are gated under the
ADR-072 v2 prelude contract: a bare call fails name resolution with
**E0100** plus a migration hint instead of reaching lint. They remain
callable during the window through an explicit `use std::prelude::{name}`
import or their canonical replacement. Deprecated entries are removed under the
[Stability Policy](stability-policy.md#deprecation-process). During 0.x, `stable`
removals may happen earlier under the explicit
[ADR-087 exception](#0x-deprecation-exception-adr-014-addendum).

---

## 0.x deprecation exception (ADR-014 addendum)

Arukellt does not cut 1.0.0 yet. While the project version is 0.x, a **`stable`**
stdlib API may be removed earlier than the normal ADR-014 deprecation window only
through the explicit exception recorded in
[ADR-087](../adr/ADR-087-0x-deprecation-exception.md). Every removal wave must
satisfy all of:

1. **W0009** is implemented and fires for the API usage.
2. This guide records the before/after and a reachable replacement (or states that
   the functionality retires).
3. [changelog.md](changelog.md) records the date, the ADR-087 reference, the target
   symbols, the removal reason, and the replacement.

Under the exception, the manifest `remove_in` value is an **upper bound**: removal
may happen earlier, but not later. The exception is 0.x-only; from 1.0.0 onward the
normal policy applies and the exception is void. The exception is not a blanket
"stdlib is provisional" cutover: each wave must name its symbols and record the
three items above. See [Stability Policy](stability-policy.md#deprecation-process)
and the [changelog](changelog.md).

## Stable-64 deprecation wave (ADR-087, 2026-10-26)

The [#917](https://github.com/wogikaze/arukellt/issues/917) wave converted the
`stable` `must delete` families listed by [ADR-087](../adr/ADR-087-0x-deprecation-exception.md)
to `stability = "deprecated"` with `deprecated_by`, `deprecated_since = "0.1.0"`,
and `remove_in = "1.0.0"` (60 entries; `i8_to_i32` is an individual deferral). The bare names emit **W0009** while they remain
resolvable; the gated prelude names also need an explicit
`use std::prelude::{name}` before the warning can fire. `remove_in` is an upper
bound under ADR-087, not the earliest removal. Physical deletion is Wave 2: the
`vec` intrinsic-compat names are compiler lowering targets used throughout the
compiler/std sources, and the `i64` / `f64` / `String` iterator lanes listed as
*planned* below are not callable yet.

Three stale, non-callable builtin manifest entries were retired directly in the
same wave because they had no declaration and no resolver registration:
`clear`, `map_option_String_String`, and `map_result_i32_i32`. Their method /
`match` replacements are shown below; old sources get an **E0100** removed-name
hint.

### Removed in the same wave

Three stale builtin entries had no declaration and no resolver registration, so
no call could reach them. They were retired directly instead of being
deprecated; old sources get an **E0100** removed-name hint.

| Removed API | Replacement | Basis |
|-------------|-------------|-------|
| `clear(v)` | `v.clear()` | stale non-callable builtin entry (ADR-087 wave) |
| `map_option_String_String(o, f)` | `match o { Some(v) => Some(f(v)), None => None }` | stale non-callable builtin entry |
| `map_result_i32_i32(r, f)` | `match r { Ok(v) => Ok(f(v)), Err(e) => Err(e) }` | stale non-callable builtin entry |

The earlier #917 slice removed the 2 `experimental` `std::wasm` helpers
(`memory_copy` / `memory_fill`, non-public `__intrinsic_*` bridge only) and the
4 gated deprecated prelude bindings (`concat`, `char_at`, `print`, `println`,
E0100 hints retained). Those removals were made under ADR-014 D4 / the 0.x
exception before ADR-087; their migration rows remain in
[`wasm.md`](modules/wasm.md) and the prelude documentation.

### Numeric conversions (16 deprecated + 1 deferral)

| Deprecated API | Replacement |
|----------------|-------------|
| `u8_to_i32(x)` | `(x).to_i32()` |
| `u16_to_i32(x)` | `(x).to_i32()` |
| `i8_to_i32(x)` | **deferred**: no receiver method yet (the `i8` method frame traps under wasm32-gc); the free remains stable |
| `i16_to_i32(x)` | `(x).to_i32()` |
| `i32_to_u8(x)` | `(x).to_u8_wrapping()` |
| `i32_to_u16(x)` | `(x).to_u16_wrapping()` (method and free currently share the deferred-body trap) |
| `i32_to_i8(x)` | `(x).to_i8_wrapping()` |
| `i32_to_i16(x)` | `(x).to_i16_wrapping()` (method and free currently share the deferred-body trap) |
| `u32_to_u64(x)` | `(x).to_u64()` |
| `u64_to_u32(x)` | `(x).to_u32_wrapping()` (method and free currently share the deferred-body trap) |
| `i32_to_i64(x)` | `(x).into()` |
| `i64_to_i32(x)` | `(x).to_i32_checked()` / `to_i32_wrapping()` / `to_i32_saturating()` |
| `f32_to_f64(x)` | `(x).to_f64()` |
| `f64_to_f32(x)` | `(x).to_f32()` |
| `i32_to_u32(x)` | `(x).to_u32_bits()` |
| `f64_bits_lo(x)` | `(x).to_bits_lo()` |
| `f64_bits_hi(x)` | `(x).to_bits_hi()` |

### Vec intrinsic-compat frees (10)

| Deprecated API | Replacement |
|----------------|-------------|
| `push(v, x)` | `v.push(x)` |
| `pop(v)` | `v.pop()` |
| `len(v)` | `v.len()` |
| `get(v, i)` | `v.get(i)` |
| `set(v, i, x)` | `v.set(i, x)` |
| `get_unchecked(v, i)` | `v.get_unchecked(i)` |
| `clear(v)` | `v.clear()` — stale non-callable entry, retired |
| `as_slice(v, s, e)` | `v.as_slice(s, e)` |
| `slice_len(view)` | `view.len()` |
| `slice_get(view, i)` | `view.get(i)` |

### Assertion frees (4)

`test` below is `std::test`; import it with `use std::test`.

| Deprecated API | Replacement |
|----------------|-------------|
| `assert_eq(a, b)` | `test::assert_eq_i32(a, b)` |
| `assert_ne(a, b)` | `test::assert_ne_i32(a, b)` |
| `assert_eq_i64(a, b)` | `test::assert_eq_i64(a, b)` |
| `assert_eq_str(a, b)` | `test::assert_eq_string(a, b)` |

### Monomorphic seq/combinator frees (33)

The `Vec` / iterator replacements are the target shape. The `i32` and `String`
producer adapters are the verified lanes; rows marked *planned* describe the
`i64` / `f64` / `String` consumer lanes that are pending. `filter_i32` is
provisional and remains an individual deferral, not part of this table.

| Deprecated API | Replacement |
|----------------|-------------|
| `sort_i32(v)` | `v.sort_by(cmp)` |
| `sort_i64(v)` | `v.sort_by(cmp)` |
| `sort_f64(v)` | `v.sort_by(cmp)` |
| `sort_String(v)` | `v.sort_by(cmp)` |
| `map_i32_i32(v, f)` | `v.iter().map(f).collect()` |
| `map_i64_i64(v, f)` | `v.iter().map(f).collect()` (i64 lane planned) |
| `map_f64_f64(v, f)` | `v.iter().map(f).collect()` (f64 lane planned) |
| `map_String_String(v, f)` | `v.iter().map(f).collect()` |
| `filter_i64(v, f)` | `v.iter().filter(f).collect()` (i64 lane planned) |
| `filter_f64(v, f)` | `v.iter().filter(f).collect()` (f64 lane planned) |
| `filter_String(v, f)` | `v.iter().filter(f).collect()` |
| `fold_i32_i32(v, init, f)` | `v.iter().fold(init, f)` |
| `fold_i64_i64(v, init, f)` | `v.iter().fold(init, f)` (i64 lane planned) |
| `fold_f64_f64(v, init, f)` | `v.iter().fold(init, f)` (f64 lane planned) |
| `any_i32(v, f)` | `v.iter().any(f)` |
| `any_i64(v, f)` | `v.iter().any(f)` (i64 lane planned) |
| `any_f64(v, f)` | `v.iter().any(f)` (f64 lane planned) |
| `any_String(v, f)` | `v.iter().any(f)` (String lane planned) |
| `find_i32(v, f)` | `v.iter().find(f)` |
| `find_i64(v, f)` | `v.iter().find(f)` (i64 lane planned) |
| `find_f64(v, f)` | `v.iter().find(f)` (f64 lane planned) |
| `find_String(v, f)` | `v.iter().find(f)` (String lane planned) |
| `contains_i32(v, x)` | `v.iter().contains(x)` |
| `contains_String(v, x)` | `v.iter().contains(x)` |
| `reverse_i32(v)` | `v.iter().rev().collect()` |
| `reverse_String(v)` | `v.iter().rev().collect()` (String rev lane planned) |
| `remove_i32(v, i)` | `v.drain(i, i + 1)` |
| `sum_i32(v)` | `v.iter().sum()` |
| `product_i32(v)` | `v.iter().fold(1, mul)` |
| `map_option_i32_i32(o, f)` | `o.map(f)` via `use std::core::container`, or a `match` |
| `map_option_String_String(o, f)` | stale non-callable entry, retired; use a `match` |
| `map_result_i32_i32(r, f)` | stale non-callable entry, retired; use a `match` |
| `contains_string(s, sub)` | `s.contains(sub)` |

**Before:**

<!-- skip-doc-check reason="migration shape example; not fixture-backed" owner="#917" kind="non-runnable" expires="2026-12-31" -->
```ark
let doubled = map_i32_i32(values, double)
let total = sum_i32(values)
sort_i32(values)
push(values, 4)
let msg = assert_eq_i64(a, b)
```

**After:**

<!-- skip-doc-check reason="migration shape example; not fixture-backed" owner="#917" kind="non-runnable" expires="2026-12-31" -->
```ark
use std::core::container
use std::core::iter
use std::collections::vec
use std::test

let doubled = values.iter().map(double).collect()
let total = values.iter().sum()
values.sort_by(cmp)
values.push(4)
test::assert_eq_i64(a, b)
```

---

## Currently Deprecated APIs

The following public entries currently carry `deprecated_by` in
`std/manifest.toml`; they are shown with ~~strikethrough~~ in the
[API reference](reference.md).

| Deprecated API | Replacement |
|----------------|-------------|
| `char_at` | `byte_at` |
| `concat` | `std::text::concat` |
| `std::env::get_var` | `std::env::var` |
| `std::host::fs::exists` | `std::host::fs::is_readable_file` |
| `std::seq::map_i32_i32` | `v.iter().map(f).collect()` or `std::core::iter::collect_map_i32` |
| `std::seq::filter_i32` | `v.iter().filter(f).collect()` or `std::core::iter::collect_filter_i32` |
| `std::seq::take_i32` | `v.iter().take(n).collect()` or `std::core::iter::collect_take_i32` |
| `std::seq::skip_i32` | `v.iter().skip(n).collect()` or `std::core::iter::collect_skip_i32` |
| `std::seq::fold_i32_i32` | `v.iter().fold(init, f)` or `std::core::iter::iter_fold_i32` |
| `std::seq::sum_i32` | `v.iter().sum()` or `std::core::iter::iter_sum_i32` |
| `std::seq::seq_reverse` | `v.iter().rev().collect()` or `std::core::iter::iter_reverse_i32` |
| `std::seq::seq_contains` | `v.iter().contains(target)` or `std::core::iter::iter_contains_i32` |
| `std::seq::unique` | `v.iter().unique().collect()` or `std::core::iter::iter_unique_i32` |
| `std::seq::count_eq` | `v.iter().count_eq(target)` or `std::core::iter::iter_count_eq_i32` |
| `std::seq::binary_search` | `std::seq::binary_search_by` |
| `std::seq::min_i32` | `std::seq::min_by` |
| `std::seq::max_i32` | `std::seq::max_by` |
| `std::collections::linear::deque_new` | `Deque::new<T>()` or `Deque::with_capacity<T>(n)` |
| `std::collections::linear::deque_push_front` | `d.push_front(v)` |
| `std::collections::linear::deque_push_back` | `d.push_back(v)` |
| `std::collections::linear::deque_pop_front` | `d.pop_front()` (`Option<T>`, `None` when empty) |
| `std::collections::linear::deque_pop_back` | `d.pop_back()` (`Option<T>`, `None` when empty) |
| `std::collections::linear::deque_front` | `d.front()` (`Option<T>`, `None` when empty) |
| `std::collections::linear::deque_back` | `d.back()` (`Option<T>`, `None` when empty) |
| `std::collections::linear::deque_len` | `d.len()` |
| `std::collections::linear::deque_is_empty` | `d.is_empty()` |
| `std::collections::linear::deque_clear` | `d.clear()` |
| `std::collections::linear::deque_to_vec` | `d.iter().collect()` |
| `std::collections::linked_list::linked_list_new` | `LinkedList::new<T>()` |
| `std::collections::linked_list::linked_list_push_front` | `l.push_front(v)` |
| `std::collections::linked_list::linked_list_push_back` | `l.push_back(v)` |
| `std::collections::linked_list::linked_list_pop_front` | `l.pop_front()` (`Option<T>`, `None` when empty) |
| `std::collections::linked_list::linked_list_pop_back` | `l.pop_back()` (`Option<T>`, `None` when empty) |
| `std::collections::linked_list::linked_list_front` | `l.front()` (`Option<T>`, `None` when empty) |
| `std::collections::linked_list::linked_list_back` | `l.back()` (`Option<T>`, `None` when empty) |
| `std::collections::linked_list::linked_list_len` | `l.len()` |
| `std::collections::linked_list::linked_list_is_empty` | `l.is_empty()` |
| `std::collections::linked_list::linked_list_clear` | `l.clear()` |
| `std::collections::linked_list::linked_list_to_vec` | `l.iter().collect()` |

The deprecated `std::seq` rows remain callable compatibility entries that
delegate to the lazy `std::core::iter` pipeline or the generic
`Ord`-based algorithms (ADR-076). Every monomorphic `std::seq` helper now
carries `deprecated_by`; the legacy `Vec_*` constructors were removed by
#845 and are no longer manifest entries.

The `deque_*` and `linked_list_*` rows are the ADR-086 linear collection
migration ([#710](https://github.com/wogikaze/arukellt/issues/710)). They keep
their monomorphic `Vec<i32>` layout for the [ADR-014](../adr/ADR-014-stability-labels.md)
deprecation window, so the observable differences from the type API are:

- Empty `deque_pop_front` / `deque_pop_back` panic on the legacy layout while
  `Deque::pop_front()` / `pop_back()` return `Option::None`.
- `deque_front` / `deque_back` return the `0` sentinel when empty while
  `Deque::front()` / `back()` return `Option::None`.
- `linked_list_pop_front` / `pop_back` / `front` / `back` take a `fallback`
  value while the `LinkedList<T>` methods return `Option<T>`.
- `deque_new` preallocates a fixed capacity of 16; `Deque::new<T>()` makes no
  capacity claim, so use `Deque::with_capacity<T>(n)` when a reservation is
  required.

New code should use `Deque<T>` / `Queue<T>` / `Stack<T>` from
`std::collections::deque` and `LinkedList<T>` from
`std::collections::linked_list`; the type surfaces carry `FromIterator`,
`Clone`, `Debug`, and `Eq` where the element type permits.

## Removed in the 0.x eradication wave (#917)

These entries were removed early under the
[0.x exception](stability-policy.md#0x-eradication-exception-917). They were
either experimental (outside the stability contract) or already gated behind
the v2 prelude with a shipped replacement.

| Removed API | Replacement | Removal basis |
|-------------|-------------|---------------|
| `std::wasm::memory_copy` | non-public `__intrinsic_memory_copy` bridge; no user-facing replacement | `experimental`, ADR-014 D4 |
| `std::wasm::memory_fill` | non-public `__intrinsic_memory_fill` bridge; no user-facing replacement | `experimental`, ADR-014 D4 |
| bare `concat(a, b)` | `std::text::concat(a, b)` or `a.concat(b)` | deprecated wrapper; E0100 hint retained |
| bare `char_at(s, i)` | `byte_at(s, i)` | deprecated wrapper; E0100 hint retained |
| bare `print(s)` | `std::host::stdio::print(s)` | deprecated binding; E0100 hint retained |
| bare `println(s)` | `std::host::stdio::println(s)` | deprecated binding; E0100 hint retained |

The `concat`, `char_at`, `print`, and `println` hints in
`src/compiler/resolver/removed_names.ark` remain active, so old sources still
get the replacement path with **E0100** rather than a bare undefined-name
error.

## Removed in the #917 core-helper wave

This wave retired the unregistered `std::core` value helpers. None of the
removed symbols had a `std::manifest.toml` entry, so no ADR-014 deprecation
window applied. The replacements are inherent methods, enum constructors, and
the `Integer` / `TryFrom` trait surface, exercised on both `wasm32-gc` and
`wasm32` by `tests/fixtures/stdlib_core/either_methods*.ark`,
`tests/fixtures/stdlib_core/integer_methods*.ark`, and
`tests/fixtures/stdlib_trait/try_from_parse*.ark`.

| Removed API | Replacement | Notes |
|-------------|-------------|-------|
| `std::core::either::left(v)` | `Either::Left(v)` | enum constructor |
| `std::core::either::right(v)` | `Either::Right(v)` | enum constructor |
| `std::core::either::is_left(e)` | `e.is_left()` | |
| `std::core::either::is_right(e)` | `e.is_right()` | |
| `std::core::either::from_left(e, fallback)` | `e.from_left(fallback)` | |
| `std::core::either::from_right(e, fallback)` | `e.from_right(fallback)` | |
| `std::core::either::left_to_option(e)` | `e.left_to_option()` | |
| `std::core::either::right_to_option(e)` | `e.right_to_option()` | |
| `std::core::either::swap(e)` | `e.swap()` | |
| `std::core::either::map_left(e, f)` | `e.map_left(f)` | |
| `std::core::either::map_right(e, f)` | `e.map_right(f)` | |
| `std::core::either::either_fold(e, l, r)` | `e.fold(l, r)` | renamed to the method shape |
| `std::core::math::gcd(a, b)` | `a.gcd(b)` | also available as the prelude `gcd` wrapper |
| `std::core::math::lcm(a, b)` | `a.lcm(b)` | new inherent `i32` method |
| `std::core::math::is_power_of_two(n)` | `n.is_power_of_two()` | `Integer` trait |
| `std::core::math::next_power_of_two(n)` | `n.next_power_of_two()` | `Integer` trait |
| `std::core::math::leading_zeros(n)` | `n.leading_zeros()` | `Integer` trait |
| `std::core::math::trailing_zeros(n)` | `n.trailing_zeros()` | `Integer` trait |
| `std::core::math::popcount(n)` | `n.popcount()` | `Integer` trait |
| `std::core::convert::parse_i32(s)` | `i32::try_from(s)` | returns `Result<i32, String>`; the free helper returned `Option` |
| `std::core::convert::parse_i64(s)` | `i64::try_from(s)` | returns `Result<i64, String>` |
| `std::core::convert::parse_f64(s)` | `f64::try_from(s)` | returns `Result<f64, String>` |

The `std::core::math` module itself remains: it still hosts the checked /
saturating arithmetic trait impls (`checked_add`, `saturating_mul`, …) used by
existing callers. Only its numeric free helpers were retired.

## Removed in the #917 WIT builder wave (2026-10-27)

This wave retired the `std::wit` world builder free functions. They were
unregistered source declarations (no `std/manifest.toml` entry), so no
ADR-014 / ADR-087 deprecation window applied, and there is no W0009 or E0100
surface to add: every removed call was module-qualified (`world::world_new`),
W0009 covers deprecated manifest entries, and E0100 covers removed bare
prelude names.

The replacements are associated functions on the builder types and receiver
methods for mutation. `std::wit::types::wit_param` retired with the same wave
because it existed only as the forwarder behind `world::wit_param_new`; the
canonical constructor is now `WitParam::new`.

| Removed API | Replacement | Notes |
|-------------|-------------|-------|
| `world::world_new(name)` | `World::new(name)` | associated function |
| `world::world_import(w, iface)` | `w.add_import(iface)` | `import` is a keyword, so the method is `add_import` |
| `world::world_export(w, iface)` | `w.add_export(iface)` | `export` is a keyword, so the method is `add_export` |
| `world::world_name(w)` | `w.name()` | |
| `world::world_exports_len(w)` | `w.exports_len()` | |
| `world::interface_new(name)` | `Interface::new(name)` | associated function |
| `world::interface_add_func(iface, func)` | `iface.add_func(func)` | |
| `world::wit_func_new(name, params, result)` | `WitFunc::new(name, params, result)` | associated function |
| `world::wit_param_new(name, ty)` | `WitParam::new(name, ty)` | defined in `std::wit::types` |
| `types::wit_param(name, ty)` | `WitParam::new(name, ty)` | retired with the builder wave |

The method surface is exercised on both targets by
`tests/fixtures/stdlib_wit/world_build.ark`,
`tests/fixtures/stdlib_wit/wit_print.ark`, and
`tests/fixtures/stdlib_wit/world_methods_wasm32.ark`.
`std::wit::world` itself remains as the owner of the `World`, `Interface`,
and `WitFunc` types.

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
Concrete source dispatch remains static, and unregistered sources still require
the corresponding concrete trait implementation. A generic function bounded by
`Iterator` can use its parameter directly as a `for` source; after
monomorphization, `iterator_vec_iter_for_loop.ark` verifies the `VecIter<i32>` case
on both backends. Generic sources without a checked iteration obligation remain
outside this migration step.
The same fixture covers a generic `T: IntoIterator` function instantiated with
`Vec<i32>`; lowering calls `Vec::into_iter` and then the canonical `VecIter<i32>::next`
sequence on both backends.
`iterator_range_basic.ark` also instantiates that generic `for` consumer with
half-open and closed range expressions, preserving their distinct boundaries.
Statically known scalar or nominal sources that implement neither
`IntoIterator` nor `Iterator`, and do not expose the structural `next` method,
are rejected during type checking with E0200. The negative contract is pinned
by `tests/fixtures/selfhost/for_deny_non_iterable.ark` and its
`.selfhost.diag` golden; unresolved generic sources remain deferred until
their concrete obligation is available.
`FromIterator` dispatches the `VecIter<i32>`, `RangeIter`, and
`RangeInclusiveIter` sources statically. A generic consumer can now return
`Vec<T::Item>` for the currently implemented `i32`-backed iterator types;
the compiler resolves that projection during monomorphization and emits the
same concrete aggregate representation on both Wasm targets. Adding iterator
implementations for other item representations is now demonstrated by the
concrete `VecIter<String>` lane, while numeric consumers remain capability-bounded
to the current `i32` implementations. `Iterator` itself is item-preserving:
`IteratorSum<Item, Output>` and `IteratorFold<Item, Accumulator>` carry the
numeric consumer contracts separately. The current
`Vec<i32>` method surface includes `iter()`, `map`, `filter`, `take`, and
`skip`; the verified composed path
`iter().map(f).filter(g).take(n).collect()` remains lazy and uses the same
canonical `Iterator::collect` traversal. Additional adapter compositions are
tracked under [#691](../../issues/open/691-iterator-trait-lazy-adapters.md)
and [#860](../../issues/open/860-iterator-unification-integration.md). The
all thirteen monomorphic `std::seq` helpers are deprecated under the
manifest contract (see [Currently Deprecated APIs](#currently-deprecated-apis)).
Calls to `sum` and `fold` now require the matching numeric capability at
type-check time. The item-preserving `VecIter<String>` lane therefore remains
valid for `next`, `collect`, `count`, and bounded adapters, while an accidental
`VecIter<String>.sum()` is rejected with E0200; it is not accepted and deferred to
an invalid backend stub. See
`tests/fixtures/selfhost/iterator_string_numeric_consumer.ark` for the
negative migration contract.
The concrete composed adapter is also a valid direct `for` source in this lane;
`iterator_adapters.ark` checks the `map/filter/take` chain through its identity
`IntoIterator::into_iter` conversion and canonical `next` path on both GC and
linear-memory Wasm. This remains a concrete fixture contract, not a claim that
arbitrary adapter types or item representations have completed migration.
The compatibility `collect_map_filter_i32` and
`collect_map_filter_take_i32` helpers now delegate to the same lazy adapter
chain and canonical `collect` traversal, so the eager entry points no longer
own duplicate iteration logic.
The adapter fixture also passes a composed `map/filter/take` source through a
generic `T: IntoIterator` `for` consumer; the identity conversion and `next`
sequence remain the same on GC and linear-memory Wasm.
The same fixture covers `iter().map(f).take(n).collect()` through the
lazy `TakeIter<MapIter>` adapter, including its canonical `collect` traversal on both
targets. This adds one concrete composition; arbitrary adapter combinations
and other item representations remain migration work.
The fixture also covers `iter().filter(g).take(n).collect()` through the
lazy `TakeIter<FilterIter>` adapter and a generic `T: IntoIterator` consumer
on both targets. This remains concrete `Vec<i32>` evidence, not a claim for
arbitrary item representations or adapter combinations.

### RFC-011 item 37 residual audit (refreshed 2026-09-26; status updated 2026-10-23)

RFC-011 item 37 asks that eager helpers, lazy adapters, `IntoIterator`, and
`FromIterator` share one trait surface. Audited against the current tree:

**Done and verified on both `wasm32-gc` and `wasm32`:**

- `Iterator` (`Item`, `next`, `collect`, `count`), `IntoIterator`,
  `FromIterator<T>`, and the `IteratorSum` / `IteratorFold` /
  `IteratorContains` / `IteratorCountEq` capability traits are defined in
  `std::core::iter` (ADR-076 producer/consumer split).
- Concrete lanes: `Vec<i32>`, `Vec<String>`, `Range`, `RangeInclusive`
  implement `IntoIterator` through the unified item-generic
  `VecIter<T>`; lazy adapters now cover map/filter/take/skip, the
  enumerated compositions, chain, enumerate, zip, flat_map, peek,
  step_by, scan (i32 and String lanes), unique, and rev; `collect` is the
  canonical trait path (`Vec<Self::Item>`).
- `for key in map` iterates every supported HashMap pair through the
  parametric `impl<K,V> IntoIterator for HashMap<K,V>` (keys snapshot
  into `VecIter<K>`); #691 closed with all thirteen `std::seq`
  helpers carrying `deprecated` + `deprecated_by` targets.
- `for` lowering dispatches concrete `IntoIterator` sources through the
  same conversion + canonical `next` sequence as handwritten consumers;
  generic `T: Iterator` / `T: IntoIterator` consumers monomorphize to the
  same path. Non-iterable sources are rejected with E0200.
- Method-level generics (ADR-079 / #887) infer the item type from the
  callback: `VecIter<T>::map<U>` returns `MapIter<T, U>` and
  `FilterIter<T>::map<U>` returns `FilterMapIter<T, U>`. A concrete-receiver
  call that leaves `U` unresolved is a type error; explicit turbofish method
  type arguments remain out of scope.
- HashMap `values()` and `entries()` (ADR-066 / #888) expose eager snapshots
  for every supported pair as `Vec<V>` / `Vec<(K, V)>`; bucket order is
  implementation-defined and documented in spec §9.14.
- Generic `ScanIter<T>` is verified beyond the i32 lane:
  `tests/fixtures/stdlib_trait/iterator_scan_string.ark` exercises
  `VecIter<String>::scan` with a `fn(String, String) -> Option<String>`
  callback and passes on both backends. The #881 Option-record-field
  type-index fix resolved the earlier bare-`T` fn-typed field concern.

**Residual (open, with owner):**

- Bounded static dispatch: the `MapIter<T, U>` / `FilterIter<T>` /
  `MapFilterIter<T, U>` / `FilterMapIter<T, U>` / `TakeIter<I>` family is
  item-generic and shared by the i32 and String lanes, but every
  instantiation is still statically monomorphized; `dyn Trait` remains out of
  scope (ADR-076). The skip compositions share the item-generic
  `SkipIter<I>` / `SkipMapIter<I>` / `SkipFilterIter<I>` adapters whose
  callbacks spell `fn(I::Item) -> I::Item`; the former concrete
  `StringSkip*` types are removed. A map callback that changes the item
  type now uses the method-level `map<U>` form above.
- Generic `for` source dispatch: a source whose iteration obligation is
  not statically resolved stays rejected. The structural `next()` fallback
  for custom iterators (`iterator/custom_iterator.ark`) remains the ADR-014
  compatibility path until its removal window closes.
- Prelude monomorphic higher-order helpers (`map_i32_i32`,
  `filter_String`, …) remain user-reachable for the ADR-014 deprecation
  window even though lazy `Iterator` replacements exist. Their retirement is
  tracked by #703/#917.
- `verify quick` exit 0 is still gated by pre-existing failures outside the
  iterator work (#765 docs hard gates, the #686 T3 fixture validation gate,
  and code-quality ratchet drift); the #880 close audit records the last
  observed set.

Dynamic dispatch (`dyn Trait`) is intentionally out of scope per ADR-076.
It also covers `iter().skip(n).take(m).collect()` through `TakeIter<SkipIter>` and a
generic `T: IntoIterator` consumer. The skip-before-take state is shared by GC
and linear-memory Wasm; other item representations and arbitrary compositions
remain migration work.
The numeric lane now also covers `skip(n).map(f)` and `skip(n).filter(g)` via
`SkipMapIter` and `SkipFilterIter`. The fixture checks lazy `collect`,
`Vec::from_iter`, generic `T: IntoIterator`, and numeric `sum` on both
published targets. This remains concrete `Vec<i32>` evidence rather than a
claim that arbitrary adapter combinations are item-generic.
The existing `std::seq` compatibility helpers for map, filter, take, skip, and
fold now delegate to the same canonical iterator consumers. The
`stdlib_seq/seq_map_filter.ark` and `stdlib_seq/seq_take_skip_fold.ark`
fixtures compile, validate, and run with identical output on `wasm32-gc` and
linear-memory `wasm32`. This is an implementation migration receipt, not a
manifest-level deprecation: the deprecation boundary still requires the
method-level manifest contract described above.

The first non-numeric item lane is `Vec<String> -> VecIter<String>`. The
`stdlib_trait/iterator_string.ark` fixture checks direct `next`, generic
`Iterator` and `IntoIterator` consumers, item-preserving `collect` and
`count`, `Vec<String>::iter()` and `Vec<String>::from_iter`, method-shaped
`collect`, direct `for` iteration, and the bounded
`iter().take(1).collect()` pipeline. `iter()` and `into_iter()` both enter the
same `VecIter<String>` producer boundary, while `take` preserves the item through
`TakeIter<VecIter<String>>`. The `skip(n).take(m)` composition uses
`TakeIter<SkipIter<VecIter<String>>>`
and applies skip state before the bounded pull. GC component and linear-memory
Wasm compilation, validation, and runtime assertions pass with the same source
behavior. The lowering selects a typed `Option::Some` payload layout for the
item on the GC path and the corresponding reference value type on the linear
fallback; no integer packing is used. This is concrete String evidence, not a
claim that every collection or adapter has become item-generic.

The String lane now also has lazy callback adapters through the item-generic
adapter types shared with the i32 lane:
`VecIter<String>::map(fn(String) -> String)` returns
`MapIter<String, String>`, and
`VecIter<String>::filter(fn(String) -> bool)` returns `FilterIter<String>`. The
`stdlib_trait/iterator_string_callbacks.ark` fixture checks both adapters and
their canonical `collect` path on GC and linear-memory Wasm. The filter adapter
clones the callback argument because the callback owns its `String` input; the
accepted source item is then yielded with its String identity intact. GC uses
typed `call_ref` signatures (`String -> String` and the Memory64 representation
of `String -> bool`), while the fallback uses the callback table and
`call_indirect`; the source-level order and results are the same.
`FromIterator<MapIter<String, String>>` and
`FromIterator<FilterIter<String>>` impls are
also reachable through `Vec::from_iter`; source-qualified associated-function
lowering selects the matching `Vec_String::from_iter__<Source>` symbol. The
fixture verifies both calls on GC and linear-memory Wasm. A `T: Iterator` helper
in the same fixture also monomorphizes to the matching static symbol, keeping
the source-level collection semantics unchanged. This remains bounded static
monomorphization evidence, not a claim for arbitrary item representations or
dynamic dispatch. The
fixture also covers `iter().map(map_identity).filter(keep_long_string).collect()`
and its `Vec::from_iter` form through `MapFilterIter<String, String>`; the
predicate sees a cloned mapped handle and accepted items retain their source
order.
The reverse `iter().filter(keep_long_string).map(map_identity)` pipeline and
its `Vec::from_iter` form are covered through `FilterMapIter<String>`;
filtering clones the source handle, then mapping consumes only accepted
originals. Method-level generics (ADR-079) infer the item type from the
callback: `VecIter<T>::map<U>` returns `MapIter<T, U>` and
`FilterIter<T>::map<U>` returns `FilterMapIter<T, U>`. A concrete-receiver
call that leaves `U` unresolved is a type error; explicit turbofish method
type arguments remain out of scope.
The same fixture passes that composed String source to a generic
`total_string_lengths<T: IntoIterator>` consumer. The emitted
`total_string_lengths__FilterMapIter_String` specialization crosses the
adapter's identity `IntoIterator` conversion and canonical `next` loop on both
published Wasm targets. This is bounded evidence for one non-numeric adapter
composition; arbitrary dynamic item dispatch remains out of scope, and
manifest-level deprecation remains for the non-deprecated `std::seq`
adapters (#703/#917).
Bounded callback compositions are also available: `map(...).take(n)` uses
`TakeIter<MapIter<String, String>>`, while `filter(...).take(n)` uses
`TakeIter<FilterIter<String>>`
and counts accepted items. The fixture verifies their lazy `collect`,
`Vec::from_iter`, and generic `T: IntoIterator` consumers on both published
Wasm targets.
The same lane supports `map(...).filter(...).take(n)` and the reverse
`filter(...).map(...).take(n)` order through
`TakeIter<MapFilterIter<String, String>>` and
`TakeIter<FilterMapIter<String>>`. Both
orders keep the predicate ownership clone and apply the bound to accepted
outputs; their `collect`, `Vec::from_iter`, and generic `for` paths are
verified on GC and linear-memory Wasm.
The lane also supports `skip(n).map(...)` and `skip(n).filter(...)` through
dedicated lazy adapters. Skipping consumes source items before callback
dispatch; filtering clones the callback argument and yields the accepted
original String. Their `collect`, `Vec::from_iter`, and generic
`T: IntoIterator` paths are verified on both targets. This is still a bounded
String migration receipt, not completion of arbitrary adapter combinations.

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

`sort<T>(v)` is not a public generic free function; the canonical shape is
the `Vec<T>` method surface from `std::collections::vec`. The comparator and
key forms landed under #697:

| Old API | Replacement |
|---------|-------------|
| `sort_i32(v)` | `v.sort_by(cmp)` / `v.sort_by_key(key)` |
| `sort_i64(v)` | `v.sort_by(cmp)` / `v.sort_by_key(key)` |
| `sort_f64(v)` | `v.sort_by(cmp)` / `v.sort_by_key(key)` |
| `sort_String(v)` | `v.sort_by(cmp)` / `v.sort_by_key(key)` |

The no-argument `v.sort()`, plus `is_sorted`, `dedup`, `binary_search_by`, and
`select_nth`, are not shipped yet. Both an `Ord`-bounded inherent impl on
`Vec<T>` and a concrete `impl Vec<i32>` lower to an unreachable body with the
current selfhost compiler, so those methods stay as the #697 residual and the
monomorphic wrappers remain available. The comparator surface that does work is
`sort_by`, `sort_by_key`, `sort_unstable_by`, `sort_unstable_by_key`,
`dedup_by`, `partition`, `is_sorted_by`, and `binary_search_by_key`. Window and
chunk helpers (`windows`, `chunks`, `chunks_exact`) return an eager
`Vec<Vec<T>>`; `resize` and `extend` copy element handles without a `Clone`
bound, and `drain` / `splice` return the removed elements as an owned `Vec<T>`.

**Before:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
sort_i32(numbers)
sort_String(names)
```

**After:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
// Comparator forms are available from std::collections::vec (#697).
numbers.sort_by(cmp)
names.sort_by(cmp)
```

### Collection Operations (map, filter, fold, etc.)

> Note: for the `i32` pipeline helpers the canonical replacement is the lazy
> `std::core::iter` method chain — for example `v.iter().map(f).collect()` —
> not a generic free function such as `map<i32, i32>(v, f)`, which does not
> exist. The 31 callable `stable` prelude monomorphic frees were deprecated in
> the [ADR-087 wave](#stable-64-deprecation-wave-adr-087-2026-10-26) (W0009,
> migration guide, changelog); two stale entries were retired and `filter_i32`
> remains deferred. The set remains owned by #703 for Wave 2 deletion.
> The provisional `filter_i32` follows an individual provisional decision.
> Rows marked *planned* describe the target method/iterator shape; they are
> not callable yet. The verified lanes are the concrete `i32` and `String`
> iterator paths documented above.

| Old API | Replacement |
|---------|-------------|
| `map_i32_i32(v, f)` | `v.iter().map(f).collect()` |
| `map_i64_i64(v, f)` | `v.iter().map(f).collect()` |
| `map_f64_f64(v, f)` | `v.iter().map(f).collect()` |
| `map_String_String(v, f)` | `v.iter().map(f).collect()` |
| `filter_i64(v, f)` | `v.iter().filter(f).collect()` |
| `filter_f64(v, f)` | `v.iter().filter(f).collect()` |
| `filter_String(v, f)` | `v.iter().filter(f).collect()` |
| `fold_i32_i32(v, init, f)` | `v.iter().fold(init, f)` |
| `fold_i64_i64(v, init, f)` | `v.iter().fold(init, f)` |
| `fold_f64_f64(v, init, f)` | `v.iter().fold(init, f)` |
| `contains_i32(v, x)` | `v.iter().contains(x)` |
| `contains_String(v, x)` | `v.iter().contains(x)` |
| `reverse_i32(v)` | `v.iter().rev().collect()` |
| `reverse_String(v)` | `v.iter().rev().collect()` |
| `remove_i32(v, idx)` | `v.remove(idx)` *(planned)* |
| `sum_i32(v)` | `v.iter().sum()` |
| `sum_i64(v)` | `v.iter().sum()` |
| `sum_f64(v)` | `v.iter().sum()` |
| `product_i32(v)` | `v.iter().fold(1, fn(acc: i32, x: i32) -> i32 { acc * x })` |
| `product_i64(v)` | `v.iter().fold(1, fn(acc: i64, x: i64) -> i64 { acc * x })` |
| `product_f64(v)` | `v.iter().fold(1, fn(acc: f64, x: f64) -> f64 { acc * x })` |
| `any_i32(v, f)` | `v.iter().any(f)` *(planned)* |
| `any_String(v, f)` | `v.iter().any(f)` *(planned)* |
| `find_i32(v, f)` | `v.iter().find(f)` *(planned)* |
| `find_String(v, f)` | `v.iter().find(f)` *(planned)* |

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
let doubled = numbers.iter().map(fn(x: i32) -> i32 { x * 2 }).collect()
let total = numbers.iter().fold(0, fn(acc: i32, x: i32) -> i32 { acc + x })
let has_it = numbers.iter().contains(42)
let reversed = numbers.iter().rev().collect()
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

The old HashMap names remain callable during the ADR-014 compatibility window. The
manifest now classifies them as `must delete` because `impl HashMap<K, V>` methods
already exist. Method-level `deprecated_by` targets such as `HashMap::insert` can now be
validated by the compat-orphan audit; adding them belongs to the #917 deprecation wave,
not to this foundation change.

### Option/Result Map

The receiver methods landed with the #917 wave in `std::core::container`:
`Option<i32>::map`, `Option<String>::map`, and `Result<i32, i32>::map`. Import
the module to use the method spelling. `map_option_String_String` and
`map_result_i32_i32` were stale non-callable builtin entries and were retired;
the `Option<i32>` method covers the callable `map_option_i32_i32` replacement.

| Old API | Replacement |
|---------|-------------|
| `map_option_i32_i32(o, f)` | `o.map(f)` via `use std::core::container`, or `match o { Some(v) => Some(f(v)), None => None }` |
| `map_option_String_String(o, f)` | retired (stale non-callable entry); use a `match` |
| `map_result_i32_i32(r, f)` | retired (stale non-callable entry); use a `match` |

**Before:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let doubled = map_option_i32_i32(maybe_val, fn(x: i32) -> i32 { x * 2 })
```

**After:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-10-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let doubled = match maybe_val {
    Some(v) => Some(v * 2),
    None => None,
}
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
| Phase 1 — Mark | **Complete** | Initial batch (`Vec_new_i32`, `Vec_new_i64`, `filter_i32`); the `Vec_new_*` names were removed by #845 |
| Phase 2 — Expand | Planned | Remaining monomorphic APIs get `deprecated_by` as generic implementations land |
| Phase 3 — Warn | **Active** | Post-resolve lint emits W0009 for deprecated API usage |
| Phase 4 — Remove | **Started** | The ADR-087 stable-64 wave deprecated 61 entries and retired 3 stale non-callable entries (#917); the 2 experimental `std::wasm` helpers and 4 gated prelude bindings were removed earlier. Physical deletion of the deprecated set is Wave 2 under the [ADR-087 exception](#0x-deprecation-exception-adr-014-addendum) |

## See Also

- [API Reference](reference.md) — deprecated entries shown with ~~strikethrough~~
- [Monomorphic Deprecation Table](monomorphic-deprecation.md) — concise status table
- [Stability Policy](stability-policy.md) — deprecation lifecycle rules
- [ADR-087: 0.x deprecation exception](../adr/ADR-087-0x-deprecation-exception.md) — conditions for early `stable` removal
- [Stdlib API Changelog](changelog.md) — recorded lifecycle changes and removal waves
- [Prelude Migration (v3)](prelude-migration.md) — historical migration context
- [Error Code W0009](../compiler/error-codes.md#w0009-deprecated-api) — deprecation warning
