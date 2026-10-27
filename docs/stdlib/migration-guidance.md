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
bound under ADR-087, not the earliest removal. Wave 2 (2026-10-27) deleted the
31 seq/combinator entries from that set; the numeric, `vec` intrinsic-compat,
and assertion families remain deprecated for their own waves. The `i64` /
`f64` / `String` iterator consumer lanes listed as *planned* below are still
not callable for every row, so the removal table names the reachable loop or
`std::seq::sort_by` fallback alongside the target method shape.

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

### Removed in Wave 2: monomorphic seq/combinator frees (31)

The 31 callable `stable` entries were deprecated in Wave 1 and deleted on
2026-10-27 under ADR-087 D2/D3 (#703 / #917). Bare calls now fail with
**E0100** and a `resolver::removed_names` hint; `filter_i32` remains
provisional (individual deferral) and is not part of this removal.

The `i32` producer/consumer lanes and the `Vec<String>` `map` / `filter` /
`any` / `find` lanes are verified on both targets. The `i64` / `f64` numeric
consumers, the `Vec<String>` reverse/contains lanes, and the generic
`Vec::sort_by` receiver for `i64` / `f64` still emit invalid Wasm in the
current emitter. Their rows give the **reachable** loop or
`std::seq::sort_by` replacement and mark the method shape they will move to.
`v.sort_by(cmp)` works for `Vec<i32>` and `Vec<String>`.

| Removed API | Replacement |
|-------------|-------------|
| `sort_i32(v)` | `v.sort_by(cmp)` |
| `sort_i64(v)` | `std::seq::sort_by(v)` (the `Vec` receiver lane is pending) |
| `sort_f64(v)` | `std::seq::sort_by(v)` (the `Vec` receiver lane is pending) |
| `sort_String(v)` | `std::seq::sort_by(v)` or `v.sort_by(cmp)` |
| `map_i32_i32(v, f)` | `v.iter().map(f).collect()` |
| `map_i64_i64(v, f)` | explicit `while` loop over `v.get_unchecked(i)` (the i64 lane is pending) |
| `map_f64_f64(v, f)` | explicit `while` loop over `v.get_unchecked(i)` (the f64 lane is pending) |
| `map_String_String(v, f)` | `v.iter().map(f).collect()` |
| `filter_i64(v, f)` | explicit `while` loop (the i64 lane is pending) |
| `filter_f64(v, f)` | explicit `while` loop (the f64 lane is pending) |
| `filter_String(v, f)` | `v.iter().filter(f).collect()` |
| `fold_i32_i32(v, init, f)` | `v.iter().fold(init, f)` |
| `fold_i64_i64(v, init, f)` | explicit `while` loop (the i64 lane is pending) |
| `fold_f64_f64(v, init, f)` | explicit `while` loop (the f64 lane is pending) |
| `any_i32(v, f)` | `v.iter().any(f)` |
| `any_i64(v, f)` | explicit `while` loop (the i64 lane is pending) |
| `any_f64(v, f)` | explicit `while` loop (the f64 lane is pending) |
| `any_String(v, f)` | `v.iter().any(f)` |
| `find_i32(v, f)` | `v.iter().find(f)` |
| `find_i64(v, f)` | explicit `while` loop (the i64 lane is pending) |
| `find_f64(v, f)` | explicit `while` loop (the f64 lane is pending) |
| `find_String(v, f)` | `v.iter().find(f)` |
| `contains_i32(v, x)` | `v.iter().contains(x)` |
| `contains_String(v, x)` | explicit `while` loop (the String lane is pending) |
| `reverse_i32(v)` | `v = v.iter().rev().collect()` |
| `reverse_String(v)` | explicit in-place swap over `v.get_unchecked` / `v.set` (the String rev lane is pending) |
| `remove_i32(v, i)` | `v.drain(i, i + 1)` |
| `sum_i32(v)` | `v.iter().sum()` |
| `product_i32(v)` | `v.iter().fold(1, mul)` |
| `map_option_i32_i32(o, f)` | `o.map(f)` via `use std::core::container`, or a `match` |
| `contains_string(s, sub)` | `s.contains(sub)` |

**Before:**

<!-- skip-doc-check reason="migration shape example; not fixture-backed" owner="#917" kind="non-runnable" expires="2026-12-31" -->
```ark
let doubled = map_i32_i32(values, double)
let total = sum_i32(values)
sort_i32(values)
```

**After:**

<!-- skip-doc-check reason="migration shape example; not fixture-backed" owner="#917" kind="non-runnable" expires="2026-12-31" -->
```ark
use std::collections::vec
use std::core
use std::core::cmp
use std::core::iter

let doubled: Vec<i32> = values.iter().map(double).collect()
let total: i32 = values.iter().sum()
values.sort_by(cmp)
```

---

## Currently Deprecated APIs

The following public entries currently carry `deprecated_by` in
`std/manifest.toml`; they are shown with ~~strikethrough~~ in the
[API reference](reference.md).

| Deprecated API | Replacement |
|----------------|-------------|
| `char_at` | `byte_at` |
| `concat` | `a.concat(b)` |
| `std::env::get_var` | `std::env::var` |
| `std::host::fs::exists` | `std::host::fs::is_readable_file` |
| `std::seq::map_i32_i32` | `v.iter().map(f).collect()` |
| `std::seq::filter_i32` | `v.iter().filter(f).collect()` |
| `std::seq::take_i32` | `v.iter().take(n).collect()` |
| `std::seq::skip_i32` | `v.iter().skip(n).collect()` |
| `std::seq::fold_i32_i32` | `v.iter().fold(init, f)` |
| `std::seq::sum_i32` | `v.iter().sum()` |
| `std::seq::seq_reverse` | `v.iter().rev().collect()` |
| `std::seq::seq_contains` | `v.iter().contains(target)` |
| `std::seq::unique` | `v.iter().unique().collect()` |
| `std::seq::count_eq` | `v.iter().count_eq(target)` |
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
| `std::io::writer_write` | `w.write(data)` on a memory `Writer`; no replacement for the tagged `stdout()` / `stderr()` handles yet |
| `std::io::writer_write_str` | `w.write_string(s)` on a memory `Writer`; no replacement for the tagged `stdout()` / `stderr()` handles yet |

The two remaining `std::io` rows are the last ADR-085 Phase F compatibility
residual ([#693](https://github.com/wogikaze/arukellt/issues/693)). They keep
W0009 coverage because `stdout()` / `stderr()` still return tagged `Vec<i32>`
handles with no `Write` implementation, so there is no method receiver to
migrate to; deletion condition: `std::host::stdio` exposes typed `Write`
handles (ADR-085 D9). Every other legacy `std::io` free function, including
`reader_from_bytes`, the seven `buf_*` entry points, `writer_new`, `write_all`,
`write_string`, `writer_flush`, and `writer_to_bytes`, was removed with an
E0100 replacement hint; see
[std::io trait migration](#stdio-trait-migration-adr-085-phase-ef-2026-10-27).

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
| bare `concat(a, b)` | `a.concat(b)` | deprecated wrapper; E0100 hint retained |
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

## Removed in the #917 WIT waves (2026-10-27)

The #693 Phase E/F merge dropped the migration rows for the first WIT wave;
they are restored here together with the second wave.

### WIT world builders (wave/917-1106)

The builder free functions were unregistered source declarations (no
`std/manifest.toml` entry), so no ADR-014 / ADR-087 deprecation window applied
and there is no W0009 surface to add. The replacements are associated
functions on the builder types plus receiver methods for mutation.

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

### WIT type / text / printer helpers (wave/917-nextgroup)

The 16 `wit_type_*` manifest entries were `experimental`, so ADR-014 D4
allowed direct deletion without a deprecation window; W0009 therefore does not
apply. The other 17 symbols were unregistered source declarations. Every
removed call was module-qualified, so the resolver returns an E0100 hint for
the `wit`, `types`, `scan`, `names`, and `printer` aliases (see
`resolver::removed_names::removed_module_member_hint`).

| Removed API | Replacement | Notes |
|-------------|-------------|-------|
| `wit::wit_type_bool()` / `types::wit_type_bool()` | `WitType::Bool` | enum variant |
| `wit::wit_type_u8()` / `types::wit_type_u8()` | `WitType::U8` | enum variant |
| `wit::wit_type_u16()` / `types::wit_type_u16()` | `WitType::U16` | enum variant |
| `wit::wit_type_u32()` / `types::wit_type_u32()` | `WitType::U32` | enum variant |
| `wit::wit_type_u64()` / `types::wit_type_u64()` | `WitType::U64` | enum variant |
| `wit::wit_type_s8()` / `types::wit_type_s8()` | `WitType::S8` | enum variant |
| `wit::wit_type_s16()` / `types::wit_type_s16()` | `WitType::S16` | enum variant |
| `wit::wit_type_s32()` / `types::wit_type_s32()` | `WitType::S32` | enum variant |
| `wit::wit_type_s64()` / `types::wit_type_s64()` | `WitType::S64` | enum variant |
| `wit::wit_type_f32()` / `types::wit_type_f32()` | `WitType::F32` | enum variant |
| `wit::wit_type_f64()` / `types::wit_type_f64()` | `WitType::F64` | enum variant |
| `wit::wit_type_char()` / `types::wit_type_char()` | `WitType::Char` | enum variant |
| `wit::wit_type_string()` / `types::wit_type_string()` | `WitType::String` | enum variant |
| `wit::wit_type_list(inner)` / `types::wit_type_list(inner)` | `WitType::list(inner)` | associated function |
| `wit::wit_type_option(inner)` / `types::wit_type_option(inner)` | `WitType::option(inner)` | associated function |
| `wit::wit_type_id(ty)` / `types::wit_type_id(ty)` | `ty.id()` | receiver method |
| `wit::wit_type_from_id(id)` / `types::wit_type_from_id(id)` | `WitType::from_id(id)` | associated function |
| `wit::wit_type_name(ty)` / `types::wit_type_name(ty)` | `ty.name()` | receiver method |
| `types::wit_type_from_ast(ty)` | `ty.wit_text()` | receiver method; also replaces the private printer duplicate |
| `types::wit_type_name_from_ark(name)` | `WitType::name_from_ark(name)` | associated function |
| `scan::wit_find_char(s, ch, from)` | `scan::WitText::find_char(s, ch, from)` | namespace associated function |
| `scan::wit_trim(s)` | `scan::WitText::trim(s)` | ASCII whitespace only |
| `scan::wit_strip_version(pkg)` | `scan::WitText::strip_version(pkg)` | |
| `scan::wit_before_colon(s)` | `scan::WitText::before_colon(s)` | |
| `scan::wit_split_param(param)` | `scan::WitText::split_param(param)` | |
| `scan::wit_split_csv_items(inner)` | `scan::WitText::split_csv_items(inner)` | tracks `<`/`>` nesting |
| `scan::wit_line_start(text, pos)` | `scan::WitText::line_start(text, pos)` | |
| `scan::wit_line_end(text, pos)` | `scan::WitText::line_end(text, pos)` | |
| `names::kebab_name(name)` | `names::WitName::to_kebab(name)` | renamed for the namespace surface |
| `names::kebab_to_snake(name)` | `names::WitName::to_snake(name)` | renamed for the namespace surface |
| `names::pascal_case(name)` | `names::WitName::to_pascal(name)` | renamed for the namespace surface |
| `names::capitalize_first(name)` | `names::WitName::capitalize(name)` | renamed for the namespace surface |
| `printer::print_wit(w)` | `w.to_wit()` | receiver method |

`std::wit` itself remains as a module root and now only documents the
submodules; import `std::wit::types`, `std::wit::scan`, `std::wit::names`,
`std::wit::printer`, or `std::wit::world` directly. The method surface is
exercised on both targets by `tests/fixtures/stdlib_wit/wit_basic.ark`,
`wit_types.ark`, `wit_type_basic.ark`, `wit_names.ark`, `wit_print.ark`,
`world_build.ark`, and `wit_methods_wasm32.ark`.

### WIT parser / component helpers (wave/wit-parser-component)

The 3 `std::wit::parser` frees were unregistered source declarations; the 7
`std::component` manifest functions were `experimental`, so ADR-014 D4 allowed
direct deletion. The 5 `handle_table_i32_*` frees were unregistered source
declarations. Every removed call was module-qualified, so the resolver returns
an E0100 hint for the `parser`, `component`, and `handle` aliases. The
`std::wit::parser` module is kept as an E0100 anchor until the 1.0.0 era; it
no longer declares a callable entry point.

| Removed API | Replacement | Notes |
|-------------|-------------|-------|
| `parser::parse_full(source)` | `WitNode::parse(source)` | full WIT 1.0 AST |
| `parser::parse_full_import_text(source)` | `WitNode::import_metadata(source)` | compiler-safe String metadata |
| `parser::parse_wit(source)` | `WitNode::parse(source)` + `World::new` / `add_import` / `add_export` | the reduced World-shaped compatibility parser retires; no direct `World::parse` associated form is shipped because a `Result<World, error::Error>` return from a std associated function fails wasm32-gc Wasm validation |
| `component::canonical_abi_version()` | `Component::canonical_abi_version()` | associated function |
| `component::component_model_version()` | `Component::model_version()` | associated function |
| `handle::handle_table_new()` | `HandleTableI32::new()` | typed table constructor |
| `handle::handle_new(t, value)` | `t.insert(value)` | receiver method |
| `handle::handle_get(t, handle)` | `t.get(handle)` | receiver method |
| `handle::handle_drop(t, handle)` | `t.remove(handle)` | receiver method |
| `handle::handle_len(t)` | `t.len()` | receiver method |
| `handle::handle_table_i32_new()` | `HandleTableI32::new()` | unregistered source free |
| `handle::handle_table_i32_new_handle(t, value)` | `t.insert(value)` | unregistered source free |
| `handle::handle_table_i32_get(t, handle)` | `t.get(handle)` | unregistered source free |
| `handle::handle_table_i32_drop(t, handle)` | `t.remove(handle)` | unregistered source free |
| `handle::handle_table_i32_len(t)` | `t.len()` | unregistered source free |

Both targets are exercised by `stdlib_wit/wit_ast_parse.ark` +
`wit_ast_parse_wasm32.ark`, `stdlib_component/component_helpers.ark` +
`component_helpers_wasm32.ark`, and `stdlib_component/handle_basic.ark` +
`handle_basic_wasm32.ark`.

`std::component::canonical` is not part of this migration yet: its
string-free functions return `Result<String, error::Error>`, and the
associated/method form of that return shape produces empty/NUL output on both
targets. The 7 frees stay until the compiler's `Result<String, Error>`
method-return lowering is fixed; see the residual note in
[`data/stdlib-api-eradication.toml`](../data/stdlib-api-eradication.toml).

## Removed in the #917 eager-iterator wave (2026-10-27)

The 44 eager `std::core::iter` constructor and collector free functions were
retired. They were `provisional` manifest entries, so ADR-014 required a
changelog notice and a documented replacement, but no deprecation window;
W0009 therefore does not apply. The replacement is the lazy adapter method
surface on `VecIter<T>`: `chain`, `enumerate`, `zip`, and `all` landed in the
same change so the whole family is expressible as method chains. Old
module-qualified calls fail resolution with an **E0100** hint from
`resolver::removed_names`.

| Removed API | Replacement |
|-------------|-------------|
| `iter::vec_iter(v)` | `v.iter()` |
| `iter::range_iter(r)` | `r.into_iter()` |
| `iter::range_inclusive_iter(r)` | `r.into_iter()` |
| `iter::map_iter(v, f)` | `v.iter().map(f)` |
| `iter::filter_iter(v, f)` | `v.iter().filter(f)` |
| `iter::take_iter(v, n)` | `v.iter().take(n)` |
| `iter::skip_iter(v, n)` | `v.iter().skip(n)` |
| `iter::chain_iter(a, b)` | `a.iter().chain(b.iter())` |
| `iter::enumerate_iter(v)` | `v.iter().enumerate()` |
| `iter::zip_iter(a, b)` | `a.iter().zip(b.iter())` |
| `iter::flat_map_iter(v, f)` | `v.iter().flat_map(f)` |
| `iter::peek_iter(v)` | `v.iter().peekable()` |
| `iter::step_by_iter(v, n)` | `v.iter().step_by(n)` |
| `iter::scan_iter(v, init, f)` | `v.iter().scan(init, f)` |
| `iter::unique_iter(v)` | `v.iter().unique()` |
| `iter::rev_iter(v)` | `v.iter().rev()` |
| `iter::collect_vec_iter(it)` | `it.collect()` |
| `iter::collect_map_iter(it)` | `it.collect()` |
| `iter::collect_filter_iter(it)` | `it.collect()` |
| `iter::collect_take_iter(it)` | `it.collect()` |
| `iter::collect_skip_iter(it)` | `it.collect()` |
| `iter::collect_chain_iter(it)` | `it.collect()` |
| `iter::collect_flat_map_iter(it)` | `it.collect()` |
| `iter::collect_peek_iter(it)` | `it.collect()` |
| `iter::collect_step_by_iter(it)` | `it.collect()` |
| `iter::collect_scan_iter(it)` | `it.collect()` |
| `iter::collect_range_iter(it)` | `it.collect()` |
| `iter::collect_map_i32(v, f)` | `v.iter().map(f).collect()` |
| `iter::collect_filter_i32(v, f)` | `v.iter().filter(f).collect()` |
| `iter::collect_take_i32(v, n)` | `v.iter().take(n).collect()` |
| `iter::collect_skip_i32(v, n)` | `v.iter().skip(n).collect()` |
| `iter::collect_chain_i32(a, b)` | `a.iter().chain(b.iter()).collect()` |
| `iter::collect_range_i32(r)` | `r.into_iter().collect()` |
| `iter::collect_map_filter_i32(v, f, g)` | `v.iter().map(f).filter(g).collect()` |
| `iter::collect_map_filter_take_i32(v, f, g, n)` | `v.iter().map(f).filter(g).take(n).collect()` |
| `iter::iter_fold_i32(v, init, f)` | `v.iter().fold(init, f)` |
| `iter::iter_count_i32(v)` | `v.iter().count()` |
| `iter::iter_sum_i32(v)` | `v.iter().sum()` |
| `iter::iter_any_i32(v, f)` | `v.iter().any(f)` |
| `iter::iter_all_i32(v, f)` | `v.iter().all(f)` |
| `iter::iter_contains_i32(v, t)` | `v.iter().contains(t)` |
| `iter::iter_count_eq_i32(v, t)` | `v.iter().count_eq(t)` |
| `iter::iter_unique_i32(v)` | `v.iter().unique().collect()` |
| `iter::iter_reverse_i32(v)` | `v.iter().rev().collect()` |

The migrated fixtures (`tests/fixtures/stdlib_core/iter_*.ark` and
`stdlib_trait/iterator_*.ark`) and the new
`stdlib_trait/iterator_method_surface*.ark` pair cover the method surface on
both `wasm32-gc` and linear-memory `wasm32`.

## Removed in the #917 std::text wave (2026-10-27)

### Removed now (experimental / unregistered)

These calls are gone. Qualified calls fail resolution with an E0100 hint
from `resolver::removed_names`.

| Removed | Replacement |
|---------|-------------|
| `rope::rope_new()` | an empty string literal |
| `rope::rope_from_string(s)` | `s.clone()` |
| `rope::rope_len(r)` | `r.len()` |
| `rope::rope_to_string(r)` | `r.clone()` |
| `rope::rope_insert(r, pos, text)` | `r.insert_text(pos, text)` |
| `rope::rope_delete(r, start, end)` | `r.delete_range(start, end)` |
| `rope::rope_slice(r, start, end)` | `r.slice(start, end)` |
| `rope::rope_line_count(r)` | `r.line_count()` |
| `fmt::format_i32(n)` | `(n).to_string()` |
| `fmt::format_i64(n)` | `(n).to_string()` |
| `fmt::format_f64(n)` | `(n).to_string()` |
| `fmt::format_bool(b)` | `(b).to_string()` |
| `fmt::pad_left(s, width, fill)` | `s.pad_left_with(width, fill)` |
| `fmt::pad_right(s, width, fill)` | `s.pad_right_with(width, fill)` |
| `fmt::format_debug(v)` | `v.fmt_debug()` |
| `fmt::write_debug_to(buf, v)` | `buf.concat(v.fmt_debug())` |
| `fmt::debug_format_vec_i32(v)` | `fmt::DebugFormat::vec_i32(v)` |
| `fmt::debug_format_vec_i64(v)` | `fmt::DebugFormat::vec_i64(v)` |
| `fmt::debug_format_vec_f64(v)` | `fmt::DebugFormat::vec_f64(v)` |
| `fmt::debug_format_vec_bool(v)` | `fmt::DebugFormat::vec_bool(v)` |
| `fmt::debug_format_vec_string(v)` | `fmt::DebugFormat::vec_string(v)` |
| `fmt::debug_format_option_i32(o)` | `fmt::DebugFormat::option_i32(o)` |
| `fmt::debug_format_option_string(o)` | `fmt::DebugFormat::option_string(o)` |
| `fmt::debug_format_result_i32_string(r)` | `fmt::DebugFormat::result_i32_string(r)` |
| `fmt::debug_format_result_string_string(r)` | `fmt::DebugFormat::result_string_string(r)` |

The six `fmt::format_*` / `fmt::pad_*` entries above were duplicate
declarations of the corresponding `std::text` manifest entries, so they are
not counted as additional removals in the eradication scorecard.

### Removed in Wave 2 (stable)

Wave 1 added the method/associated replacements and made these 36 `stable`
entries `deprecated` with W0009 coverage for qualified `text::*` calls. The
Wave 2 deletion removed the manifest entries and declarations; qualified calls
now fail with an E0100 hint from `resolver::removed_names`. The same change
removed the seven duplicate `std::collections::string` frees (`concat`,
`chars`, `len_chars`, `codepoint_at`, `try_slice_bytes`, `slice_chars`,
`try_slice_chars`) that became unregistered when the `std::text` entries
retired; their `string::*` spellings also carry E0100 hints.

| Removed | Replacement |
|---------|-------------|
| `text::len_bytes` | `s.len_bytes()` |
| `text::is_empty` | `s.is_empty()` |
| `text::index_of` | `s.index_of(sub)` |
| `text::trim` | `s.trim()` |
| `text::trim_start` | `s.trim_start()` |
| `text::trim_end` | `s.trim_end()` |
| `text::replace` | `s.replace(from, to)` |
| `text::lines` | `s.lines()` |
| `text::chars` | `s.chars()` |
| `text::pad_left` | `s.pad_left_with(width, fill)` |
| `text::pad_right` | `s.pad_right_with(width, fill)` |
| `text::format_i32` | `(n).to_string() on i32` |
| `text::format_i64` | `(n).to_string() on i64` |
| `text::format_f64` | `(n).to_string() on f64` |
| `text::format_bool` | `(b).to_string() on bool` |
| `text::repeat` | `s.repeat(n)` |
| `text::len_chars` | `s.len_chars()` |
| `text::codepoint_at` | `s.codepoint_at(i)` |
| `text::slice_bytes` | `s.slice_bytes(start, end)` |
| `text::try_slice_bytes` | `s.try_slice_bytes(start, end)` |
| `text::slice_chars` | `s.slice_chars(start, end)` |
| `text::try_slice_chars` | `s.try_slice_chars(start, end)` |
| `text::split` | `s.split(sep)` |
| `text::join` | `parts.join(sep)` |
| `text::concat` | `a.concat(b)` |
| `text::starts_with` | `s.starts_with(prefix)` |
| `text::ends_with` | `s.ends_with(suffix)` |
| `text::contains` | `s.contains(sub)` |
| `text::to_lower` | `s.to_lower()` |
| `text::to_upper` | `s.to_upper()` |
| `text::builder_new` | an empty string literal |
| `text::builder_append` | `s.concat(text)` |
| `text::builder_append_char` | `s.concat(c.to_string())` |
| `text::builder_append_line` | `s.concat(text).concat("\n")` |
| `text::builder_build` | `the builder value directly` |
| `text::builder_len` | `s.len()` |


## std::simd lane namespace wave (ADR-046, 2026-10-27)

The `std::simd` lane modules were `experimental` free-function modules. The
wave moves the 132 operations whose emitter lowering is valid on both
`wasm32-gc` and `--target wasm32` to one namespace type per lane module. The
old qualified call shape maps directly:

```ark
// before
use std::simd::f32x4
let v = f32x4::add(f32x4::splat(1.0f32), f32x4::splat(2.0f32))

// after
use std::simd::f32x4::F32x4
let v = F32x4::add(F32x4::splat(1.0f32), F32x4::splat(2.0f32))
```

The `[[functions]]` entries for the migrated operations are gone and their
`[[types]]` method lists are the manifest surface. Because the entries were
`experimental`, ADR-014 D4 allows direct deletion; no W0009 deprecation window
was opened and no `deprecated_by` entry was added. A qualified old call such as
`f32x4::splat(...)` fails resolution with an **E0100** hint that names
`F32x4::splat(...)` and the import path.

| Module | Namespace type | Associated functions (migrated) |
|--------|----------------|----------------------------------|
| `std::simd::f32x4` | `F32x4` | all except `neg` |
| `std::simd::f64x2` | `F64x2` | `splat`, `new`, `zero`, `extract_lane`, `replace_lane`, `abs`, `eq`, `ne`, `lt`, `le`, `gt`, `ge`, `ceil`, `floor`, `trunc`, `nearest`, `promote_low_f32x4`, `convert_low_i32x4_s`, `convert_low_i32x4_u` |
| `std::simd::i8x16` | `I8x16` | `splat`, `new`, `zero`, `extract_lane`, `replace_lane`, `abs`, `eq`, `ne`, `lt_s`, `gt_s`, `lt_u`, `gt_u`, `min_s`, `min_u`, `max_s`, `max_u`, `shl`, `shr_s`, `shr_u`, `all_true`, `bitmask`, `narrow_i16x8_s`, `narrow_i16x8_u` |
| `std::simd::i16x8` | `I16x8` | `splat`, `new`, `zero`, `extract_lane`, `replace_lane`, `add`, `sub`, `ge_u`, `min_s`, `min_u`, `max_s`, `max_u`, `shl`, `shr_s`, `shr_u`, `extadd_pairwise_i8x16_s`, `extadd_pairwise_i8x16_u`, `extend_low_i8x16_s`, `extend_high_i8x16_s`, `extend_low_i8x16_u`, `extend_high_i8x16_u`, `narrow_i32x4_s`, `narrow_i32x4_u` |
| `std::simd::i32x4` | `I32x4` | all except `max_u` |
| `std::simd::i64x2` | `I64x2` | `splat`, `new`, `zero`, `extract_lane`, `replace_lane`, `eq`, `ne`, `lt_s`, `le_s`, `ge_s` |
| `std::simd::v128` | `V128` | `zero`, `and`, `andnot`, `or`, `xor`, `not`, `bitselect`, `any_true` |
| `std::simd::u8x16` / `u16x8` / `u32x4` / `u64x2` | — | none; every operation still fails Wasm validation |

### Retained: lowering-blocked frees (130)

The other 130 operations stay as experimental free functions. A method
replacement is not advertised for an operation that cannot run:

- **98 fail Wasm validation on both targets** (the complete `u8x16`, `u16x8`,
  `u32x4`, and `u64x2` modules, plus `f64x2::{neg, sqrt}`,
  `i8x16::{neg, sub_sat_u}`, `i16x8::{neg, abs, add_sat_s, add_sat_u,
  sub_sat_s, sub_sat_u}`, and `i64x2::{add, neg, abs, shl, shr_s, shr_u}`).
  The exact `wasm-tools validate` diagnostics are recorded in issue #917;
  the emitter either emits a missing operand (`expected v128 but nothing on
  the stack`), an unknown `0xfd` subopcode, or an i64/i32 lane-width
  mismatch.
- **32 validate but trap or return the wrong value**:
  `f32x4::neg` (returns the input), `f64x2::{add, sub, mul, div}` (return the
  first operand), `f64x2::{min, max}` (trap in an unresolved call),
  `i8x16::{add, sub, mul, add_sat_s, add_sat_u, sub_sat_s}` (wrong lane
  values), `i8x16::{le_s, le_u, ge_s, ge_u}` (trap), `i16x8::mul` (returns
  the input), `i16x8::{eq, ne, lt_s, le_s, gt_s, ge_s, lt_u, le_u, gt_u}`
  (trap), `i32x4::max_u` (trap), `i64x2::{sub, mul}` (return the input),
  `i64x2::gt_s` (trap), and `v128::new` (returns an all-zero vector).

The per-op evidence is the wave receipt on issue #917. The existing
`simd_conformance` / `simd_lowering` / `simd_gc_storage` / `v128_basic`
fixtures were migrated to the associated spelling; the new
`stdlib_simd/simd_method_surface.ark` + `_wasm32.ark` pair asserts the values
of the migrated surface on both targets, and
`diagnostics/removed_simd_free.ark` + `_wasm32.ark` pin the E0100 hint.

## std::bytes method wave (ADR-087, 2026-10-27)

The [#917](https://github.com/wogikaze/arukellt/issues/917) bytes wave landed
the receiver / associated surface for the shared `Vec<i32>` byte layout and
made the 45 registered `stable` free functions `deprecated` with W0009,
`deprecated_since = "0.1.0"`, and `remove_in = "1.0.0"`. They stay callable
through the 0.x window; the method surface is the replacement. The eight
unregistered helpers (`hex_char`, `hex_val_char`, `try_hex_val_char`, and the
five `try_read_*` readers) were retired immediately with E0100 hints because
they had no manifest lifecycle entry.

`Bytes`, `ByteBuf`, and `ByteCursor` are still `Vec<i32>` at runtime. Import
`use std::bytes` for the methods and `use std::bytes::Bytes` for the
receiver-less constructors/decoders.

| Deprecated free | Replacement |
|-----------------|-------------|
| `bytes::bytes_new` | `Vec::new<i32>()` |
| `bytes::bytes_from_string` | `s.to_utf8_bytes()` |
| `bytes::string_from_bytes` | `String::from_utf8_lossy(b)` |
| `bytes::bytes_len` / `bytes::buf_len` | `b.len()` |
| `bytes::bytes_get` | `b.get_unchecked(i)` |
| `bytes::bytes_push` / `bytes::buf_push_u8` | `b.push_byte(x)` |
| `bytes::bytes_eq` | `a.equals_bytes(b)` |
| `bytes::bytes_concat` | `a.concat_bytes(b)` |
| `bytes::bytes_slice` | `b.slice_bytes(start, end)` |
| `bytes::byte_length` | `s.len_bytes()` |
| `bytes::hex_encode` | `b.to_hex()` |
| `bytes::hex_decode` | `Bytes::from_hex(s)` |
| `bytes::leb128_encode_u32` | `Bytes::from_leb128_u32(x)` |
| `bytes::leb128_encode_i32` | `Bytes::from_leb128_i32(x)` |
| `bytes::leb128_encode_u64` | `Bytes::from_leb128_u64(x)` |
| `bytes::u16_to_le_bytes` | `Bytes::from_u16_le(x)` |
| `bytes::u16_from_le_bytes` | `b.to_u16_le()` |
| `bytes::u32_to_le_bytes` | `Bytes::from_u32_le(x)` |
| `bytes::u32_from_le_bytes` | `b.to_u32_le()` |
| `bytes::u32_to_be_bytes` | `Bytes::from_u32_be(x)` |
| `bytes::u32_from_be_bytes` | `b.to_u32_be()` |
| `bytes::bytes_from_array` | `Bytes::from_array(xs)` |
| `bytes::buf_new` | `Vec::new<i32>()` |
| `bytes::buf_with_capacity` | `Vec::with_capacity<i32>(cap)` |
| `bytes::buf_push_u16_le` | `b.push_u16_le(x)` |
| `bytes::buf_push_u32_le` | `b.push_u32_le(x)` |
| `bytes::buf_push_u64_le` | `b.push_u64_le(x)` |
| `bytes::buf_extend` | `b.extend_bytes(other)` |
| `bytes::buf_freeze` | `b.freeze()` (identity; both layouts are `Vec<i32>`) |
| `bytes::cursor_new` | `data.byte_cursor()` |
| `bytes::cursor_pos` | `c.cursor_pos()` |
| `bytes::cursor_remaining` | `c.cursor_remaining()` |
| `bytes::read_u8` | `c.read_u8()` |
| `bytes::read_u16_le` | `c.read_u16_le()` |
| `bytes::read_u32_le` | `c.read_u32_le()` |
| `bytes::read_u64_le` | `c.read_u64_le()` |
| `bytes::read_u32_be` | `c.read_u32_be()` |
| `bytes::read_bytes` | `c.read_bytes(n)` |
| `bytes::base64_encode` | `b.to_base64()` |
| `bytes::base64_decode` | `Bytes::from_base64(s)` |
| `bytes::leb128_decode_u32` | `c.read_leb128_u32()` |
| `bytes::leb128_decode_i32` | `c.read_leb128_i32()` |
| `bytes::leb128_decode_u64` | `c.read_leb128_u64()` |
| `bytes::hex_char` (removed) | `b.to_hex()` |
| `bytes::hex_val_char` (removed) | `Bytes::from_hex(s)` |
| `bytes::try_hex_val_char` (removed) | `Bytes::from_hex(s)` |
| `bytes::try_read_u8` (removed) | `c.try_read_u8()` |
| `bytes::try_read_u16_le` (removed) | `c.try_read_u16_le()` |
| `bytes::try_read_u32_le` (removed) | `c.try_read_u32_le()` |
| `bytes::try_read_u32_be` (removed) | `c.try_read_u32_be()` |
| `bytes::try_read_bytes` (removed) | `c.try_read_bytes(n)` |

The dual-target fixtures are `stdlib_bytes/bytes_method_surface*.ark`; the
W0009 goldens are `diagnostics/deprecated_bytes_free*.ark`. The 45 deprecated
manifest entries remain until the ADR-087 deletion wave.

## std::io trait migration (ADR-085 Phase E/F, 2026-10-27)

The [ADR-085](../adr/ADR-085-io-trait-unification.md) migration replaced the
`Vec<i32>` layout conventions with the `Reader` / `Writer` newtypes, the
`Read` / `Write` / `BufRead` / `Seek` traits, the `Io` generic algorithms, the
generic `BufReader<R>` / `BufWriter<W>` adapters, and `std::host::fs::File`.
The canonical imports are `use std::io::{Reader, Writer, Io, BufReader,
BufWriter, Read, Write, BufRead, Seek, SeekFrom}` and `use std::host::fs::File`.

Removed in this wave (E0100 with a replacement hint; no manifest entry
remains):

| Removed API | Replacement |
|-------------|-------------|
| `io::reader_eof(r)` | `r.is_eof()` |
| `io::reader_read(r, n)` | `r.read(n)` |
| `io::reader_read_exact(r, n)` | `Io::read_exact(r, n)` |
| `io::reader_read_all(r)` | `Io::read_to_end(r)` |
| `io::reader_read_line(r)` | `r.read_line()` (`Option<String>`, `None` at EOF) |
| `io::copy_bytes(from, to)` / `io::copy(from, to)` | `Io::copy(from, to)` (`i64` count) |
| `io::copy_trait(from, to)` | `Io::copy(from, to)` |
| `io::new_memory_buffer()` | `Reader::new()` for a source, `Writer::new()` for a sink |
| `io::write_bytes(buf, data)` | `w.write(data)` |
| `io::read_bytes(buf, n)` | `r.read(n)` |
| `io::seek_to(buf, pos)` | `r.seek(SeekFrom::Start(pos))` |
| `io::buffer_position(buf)` | `r.position()` |
| `io::buffer_len(buf)` | `r.len()` |
| `io::fill_buffer(buf)` | `r.fill_buf()` |
| `io::file_from_fd` / `io::file_read` / `io::file_write` | `std::host::fs::File` (read side; the whole-file write side is still open under #693) |

The Phase F completion wave removed the remaining fully-replaced wrappers
(2026-10-27, [#693](https://github.com/wogikaze/arukellt/issues/693)):

| Removed API | Replacement |
|-------------|-------------|
| `io::reader_from_bytes(data)` | `Reader::from_bytes(data)` |
| `io::buf_reader_new(r, cap)` / `io::buffered_reader(r, cap)` | `BufReader::new(r, cap)` |
| `io::buf_reader_read_line(br)` | `br.read_line()` (`Option<String>`, `None` at EOF) |
| `io::buf_writer_new(w, cap)` / `io::buffered_writer(w, cap)` | `BufWriter::new(w, cap)` |
| `io::buf_writer_write_str(bw, s)` | `bw.write_string(s)` |
| `io::buf_writer_flush(bw)` | `bw.flush()` |
| `io::writer_new()` | `Writer::new()` |
| `io::write_all(w, data)` | `Io::write_all(w, data)` |
| `io::write_string(w, s)` | `w.write_string(s)` on a memory `Writer` |
| `io::writer_flush(w)` | `w.flush()` on a memory `Writer` |
| `io::writer_to_bytes(w)` | `w.into_bytes()` on a memory `Writer` |

Behavior differences to account for while migrating:

- `Read::read` returns an empty `Vec` at EOF and rejects a negative
  `max_bytes` with `IoError::Other`; the legacy `read_bytes` returned an empty
  `Vec` for a negative count.
- `BufRead::read_line` returns `Option<String>` and `None` at EOF; the legacy
  `reader_read_line` / `buf_reader_read_line` returned `Ok("")` at EOF.
- `Seek::seek` rejects a negative effective position and allows seeking past
  the end (reads then report EOF); the legacy `seek_to` clamped into
  `[0, len]`.
- `Io::read_exact` reports `UnexpectedEof` with a `need N have M` message;
  the legacy `reader_read_exact` prefixed it with `UnexpectedEof:`.
- `Io::copy` returns `i64` and stops on the first read or write error.
- `BufReader` reads ahead into its own buffer; `Read::read` on the adapter
  returns buffered bytes before touching the inner handle, so seek the inner
  handle only through `BufReader::seek`.
- The tagged `stdout()` / `stderr()` handles still have no typed `Write`
  implementation, so `io::writer_write` / `io::writer_write_str` remain the only
  way to write to them; their W0009 hint says so and does not promise a method
  call that those handles cannot satisfy.

**Before:**

<!-- skip-doc-check reason="migration shape example; not fixture-backed" owner="#693" kind="non-runnable" expires="2026-12-31" -->
```ark
let r = io::reader_from_bytes(data)
let chunk = io::reader_read_exact(r, 4)?
let w = io::writer_new()
io::write_bytes(w, chunk)
let pos = io::seek_to(w, 0)
```

**After:**

<!-- skip-doc-check reason="migration shape example; not fixture-backed" owner="#693" kind="non-runnable" expires="2026-12-31" -->
```ark
let r: Reader = Reader::from_bytes(data)
let chunk = Io::read_exact(r, 4)?
let w: Writer = Writer::new()
w.write(chunk)?
let pos = w.into_reader().seek(SeekFrom::Start(0))?
```

Evidence for the wave:

- `tests/fixtures/diagnostics/deprecated_io_free_function.ark` (W0009 live for
  the retained compatibility family).
- `tests/fixtures/diagnostics/removed_io_free_function.ark` (E0100 replacement
  hint for a removed member).
- `tests/fixtures/diagnostics/removed_io_compat_free_function.ark` (E0100 hint
  for a wrapper retired in the Phase F completion wave).
- The migrated `tests/fixtures/stdlib_io*`, `tests/fixtures/stdlib_trait/`, and
  `tests/fixtures/question_mark/error_ecosystem.ark` fixtures compile and run
  on both `wasm32-gc` and `wasm32`.
- `use std::io` no longer emits the retired "moved to `std::host::stdio`"
  W0009 warning (ADR-085 D1); `std::io` is the canonical abstraction module.

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
The existing `std::seq` compatibility helpers keep their eager signatures and
all delegate to the canonical iterator methods, including the lazy
`TakeIter` / `SkipIter` collectors (#691). The
`stdlib_seq/seq_map_filter.ark`, `stdlib_seq/seq_take_skip_fold.ark`, and
`stdlib_seq/seq_take_skip_collect.ark` fixtures compile, validate, and run
with identical output on `wasm32-gc` and linear-memory `wasm32`. This is an
implementation migration receipt, not a manifest-level deprecation: the
deprecation boundary still requires the method-level manifest contract
described above.

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

### Sort and collection operations

`sort_i32` / `sort_i64` / `sort_f64` / `sort_String` and the monomorphic
map/filter/fold/any/find/contains/reverse/remove/sum/product helpers were
removed in the ADR-087 Wave 2; see the
[removal table](#removed-in-wave-2-monomorphic-seqcombinator-frees-31).
`Vec` exposes `sort_by` / `sort_by_key` / `sort_unstable_by` /
`sort_unstable_by_key` / `dedup_by` / `partition` / `is_sorted_by` /
`binary_search_by_key`; `std::seq::sort_by` remains the generic `Ord`-based
algorithm and is the reachable sort fallback for the `i64` / `f64` / `String`
lanes until their `Vec` receiver methods lower correctly. The no-argument
`v.sort()`, `is_sorted`, `dedup`, and `select_nth` remain the #697 residual.
Window and chunk helpers (`windows`, `chunks`, `chunks_exact`) return an eager
`Vec<Vec<T>>`; `resize` and `extend` copy element handles without a `Clone`
bound, and `drain` / `splice` return the removed elements as an owned `Vec<T>`.

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
| `hash_map::hashmap_str_i32_new()` | `HashMap::new<String, i32>()` |
| `hash_map::hashmap_str_i32_insert(m, k, v)` | `m.insert(k, v)` |
| `hash_map::hashmap_str_i32_get(m, k)` | `m.get(k)` |
| `hash_map::hashmap_str_i32_contains(m, k)` | `m.contains_key(k)` |
| `hash_map::hashmap_str_i32_remove(m, k)` | `m.remove(k)` |
| `hash_map::hashmap_str_i32_len(m)` | `m.len()` |
| `hash_map::hashmap_str_i32_is_empty(m)` | `m.is_empty()` |
| `hash_map::hashmap_str_i32_keys/values/entries(m)` | `m.keys()` / `m.values()` / `m.entries()` |
| `hash_map::hashmap_i32_str_*` | the same methods on `HashMap<i32, String>` |
| `hash_map::hashmap_str_str_*` | the same methods on `HashMap<String, String>` |
| `hash_map::hashmap_i32_fn_*` / `hash_fn::hashmap_i32_fn_*` | the same methods on `HashMap<i32, fn(i32) -> i32>`; `set` becomes `insert` (the previous `Option<V>` is the return value) |
| `hash_map::hashmap_str_fn_*` / `hash_str_fn::hashmap_str_fn_*` | the same methods on `HashMap<String, fn(i32) -> i32>`; `set` becomes `insert` |
| `hash_string::hashmap_str_*` / `hash_string::hashmap_i32_str_*` / `hash_string::hashmap_str_str_*` | the matching `HashMap<K, V>` methods |

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

The typed `hash_map::` / `hash_string::` / `hash_fn::` / `hash_str_fn::` adapter
frees were retired by #917 on 2026-10-27. The `impl HashMap<K, V>` methods are the
only spelling: the compiler rewrites each supported pair directly to the
non-public implementation functions, and the old qualified names fail resolution
with an E0100 hint that points at the method surface. `insert` keeps the
`Option<V>` previous-value contract, so `let old = m.insert(k, v)` is the exact
replacement for the old `*_insert` adapters.

### HashSet<i32> Membership

`HashSet<i32>` is the canonical surface for the six basic membership
operations over the flat hash set store (#917). The aggregate
(`to_vec` / `union` / `intersection` / `difference` / `capacity` / `reserve` /
`shrink_to_fit` / `clear` / `extend`) and `HashSet<String>` lanes keep the
deprecated `hash_set::` adapters until their GC lowering lands.

| Old API | Replacement |
|---------|-------------|
| `hash_set::hashset_i32_new()` | `HashSet::new<i32>()` |
| `hash_set::hashset_i32_insert(s, v)` | `s.insert(v)` |
| `hash_set::hashset_i32_contains(s, v)` | `s.contains(v)` |
| `hash_set::hashset_i32_remove(s, v)` | `s.remove(v)` |
| `hash_set::hashset_i32_len(s)` | `s.len()` |
| `hash_set::hashset_i32_is_empty(s)` | `s.is_empty()` |

Import the type explicitly: `use std::collections::hash_set::HashSet`.
`hash_set::hashset_i32_union` / `intersection` / `difference` / `to_vec` /
`capacity` / `reserve` / `shrink_to_fit` / `clear` / `extend` and the
`hashset_str_*` facade have no method replacement yet and remain the
compatibility spelling.

**Before:**

<!-- skip-doc-check reason="historical removed-API example" owner="#917" kind="non-runnable" expires="2026-10-31" -->
```ark
let s = hash_set::hashset_i32_new()
hash_set::hashset_i32_insert(s, 1)
let n = hash_set::hashset_i32_len(s)
```

**After:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#917" kind="non-runnable" expires="2026-10-31" -->
```ark
let s: HashSet<i32> = HashSet::new<i32>()
let _ = s.insert(1)
let n = s.len()
```

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
