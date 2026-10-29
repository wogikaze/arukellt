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

## Host/core namespace retirement (ADR-092)

The [#917](https://github.com/wogikaze/arukellt/issues/917) host/core wave retired
25 `stable` free functions and moved them onto four namespace types. The three
experimental `std::host::fs` fd helpers are the only retained exception (see
below). [ADR-092](../adr/ADR-092-host-core-namespace-free-exception.md) records the
narrow ADR-087 D2 exception: the old names fail with **E0100** and a replacement
hint instead of a W0009 window, because every replacement is already callable and
verified on both targets. The exception names exactly the 25 symbols in the table.

| Removed free call | Replacement |
|-------------------|-------------|
| `std::host::process::exit(code)` | `Process::exit(code)` via `use std::host::process::Process` |
| `std::host::process::abort()` | `Process::abort()` via `use std::host::process::Process` |
| `std::host::process::id()` | `Process::id()` via `use std::host::process::Process` |
| `std::process::exit(code)` | `Process::exit(code)` via `use std::host::process::Process` |
| `std::process::abort()` | `Process::abort()` via `use std::host::process::Process` |
| `std::host::random::random_i32()` | `Random::random_i32()` via `use std::host::random::Random` |
| `std::host::random::random_i32_range(lo, hi)` | `Random::random_i32_range(lo, hi)` via `use std::host::random::Random` |
| `std::host::random::random_bool()` | `Random::random_bool()` via `use std::host::random::Random` |
| `std::host::random::next_f64()` | `Random::next_f64()` via `use std::host::random::Random` |
| `std::cli::parse_args()` | `Env::args()` via `use std::host::env::Env` |
| `std::cli::arg_count()` | `Env::arg_count()` via `use std::host::env::Env` |
| `std::cli::arg_at(index)` | `Env::arg_at(index)` via `use std::host::env::Env` |
| `std::cli::has_flag(flag)` | `Env::has_flag(flag)` via `use std::host::env::Env` |
| `std::cli::flag(args, name)` | `Cli::flag(args, name)` via `use std::cli::Cli` |
| `std::cli::option(args, name)` | `Cli::option(args, name)` via `use std::cli::Cli` |
| `std::signal::pi()` | `Signal::pi()` via `use std::signal::Signal` |
| `std::signal::tau()` | `Signal::tau()` via `use std::signal::Signal` |
| `std::signal::sin_approx(x)` | `Signal::sin_approx(x)` via `use std::signal::Signal` |
| `std::signal::cos_approx(x)` | `Signal::cos_approx(x)` via `use std::signal::Signal` |
| `std::signal::clone_f64(v)` | `Signal::clone_f64(v)` via `use std::signal::Signal` |
| `std::signal::zeros(n)` | `Signal::zeros(n)` via `use std::signal::Signal` |
| `std::signal::fft(real, imag)` | `Signal::fft(real, imag)` via `use std::signal::Signal` |
| `std::signal::ifft(real, imag)` | `Signal::ifft(real, imag)` via `use std::signal::Signal` |
| `std::signal::magnitude(real, imag)` | `Signal::magnitude(real, imag)` via `use std::signal::Signal` |
| `std::signal::approx_eq(a, b, eps)` | `Signal::approx_eq(a, b, eps)` via `use std::signal::Signal` |

`std::process` remains a compatibility facade that re-exports `Process`; importing
it still emits the existing W0002 "module moved" warning. The E0100 goldens are
`diagnostics/removed_host_process_free*`, `diagnostics/removed_process_legacy_free*`,
`diagnostics/removed_host_random_free*`, `diagnostics/removed_cli_free*`, and
`diagnostics/removed_signal_free*`.

### Retained: experimental fd helpers (3)

`fd_seek`, `fd_tell`, and `fd_fdstat_errno` stay experimental free functions.
They are `wasm32` / WASI P1 only (`t3 = false`), so they cannot be exercised on
`wasm32-gc`; moving them to `Fs` without a target-parity fixture would claim a
surface that does not lower on the primary target. They are the exact exception
list of [ADR-092](../adr/ADR-092-host-core-namespace-free-exception.md) §4 and
remain a reported residual.

## Prelude free-function retirement (ADR-091)

The [#917](https://github.com/wogikaze/arukellt/issues/917) `wave/prelude-remain`
retired 27 `stable` prelude free functions. [ADR-091](../adr/ADR-091-prelude-free-function-exception.md)
records the narrow ADR-087 D2 exception: the old names fail with **E0100** and a
replacement hint instead of a W0009 window, because every replacement is a
receiver method or associated function that is already callable and verified.
The same wave removed the 13 corresponding `std/collections/string.ark` free
declarations from the public module surface (12 became private; the duplicate
`push_char` declaration was deleted because the prelude free remains).

| Removed free call | Replacement |
|-------------------|-------------|
| `abs(x)` | `(x).abs()` |
| `args()` | `Env::args()` via `use std::host::env::Env` |
| `byte_len(s)` | `s.len_bytes()` |
| `clamp_i32(x, lo, hi)` | `(x).clamp(lo, hi)` |
| `exit(code)` | `Process::exit(code)` via `use std::host::process::Process` |
| `format_bool(b)` | `(b).to_string()` |
| `format_i32(n)` | `(n).to_string()` |
| `gcd(a, b)` | `(a).gcd(b)` |
| `is_empty(x)` | `x.is_empty()` |
| `join(parts, sep)` | `parts.join(sep)` |
| `lines(s)` | `s.lines()` |
| `max(a, b)` | `(a).max(b)` |
| `min(a, b)` | `(a).min(b)` |
| `pad_left(s, width)` | `s.pad_left(width)` |
| `pad_right(s, width)` | `s.pad_right(width)` |
| `repeat(s, n)` | `s.repeat(n)` |
| `replace(s, from, to)` | `s.replace(from, to)` |
| `split(s, sep)` | `s.split(sep)` |
| `sqrt(x)` | `(x).sqrt()` |
| `to_lower(s)` | `s.to_lower()` |
| `to_lowercase(s)` | `s.to_lower()` (ASCII case fold) |
| `to_upper(s)` | `s.to_upper()` |
| `to_uppercase(s)` | `s.to_upper()` (ASCII case fold) |
| `trim(s)` | `s.trim()` |
| `trim_end(s)` | `s.trim_end()` |
| `trim_start(s)` | `s.trim_start()` |
| `values(v)` | No replacement; iterate the value directly with `for x in v` |

The following four prelude free functions remain as the ADR-091 §3 exception:
`assert(cond)`, `String_from(s)`, `String_new()`, and `char_to_string(c)`.
They are the v2 stable Prelude set from ADR-072 §1 (after `panic` moved to
`Std::panic` in ADR-090). `String::new()` was measured to produce invalid core
Wasm on the current pin, and `char_to_string` preserves the i32-codepoint call
boundary, so these are not mechanically replaced in this wave.

The `wave/prelude-text` follow-up retired 11 of the 23 families that ADR-091 §4
had deferred. The old free calls now fail with E0100 and a replacement hint:

| Removed deferred free | Replacement |
|-----------------------|-------------|
| `push_char(s, c)` | `s.push_char(c)` (the receiver writeback covers locals and fields) |
| `unwrap(o)` | `o.unwrap()` on `Option<i32>`; `r.unwrap()` on `Result<i32, i32>` |
| `unwrap_or(o, default)` | `o.unwrap_or(default)` / `r.unwrap_or(default)` |
| `is_some(o)` / `is_none(o)` | `o.is_some()` / `o.is_none()` |
| `is_ok(r)` / `is_err(r)` | `r.is_ok()` / `r.is_err()` |
| `parse_i32(s)` | `i32::try_from(s)` via `use std::core::convert` |
| `parse_i64(s)` | `i64::try_from(s)` via `use std::core::convert` |
| `parse_f64(s)` | `f64::try_from(s)` via `use std::core::convert` |
| `f32_to_string(x)` | `x.to_string()` |

`i32::try_from` / `i64::try_from` / `f64::try_from` are associated functions from
`std::core::convert`; calling them without that `use` is E0100 with the import
hint. Receiver methods also resolve when the receiver is a call result
(`pop(v).unwrap()`, `maybe(...).is_some()`): MIR maps the `option:<T>`
call-result type to the concrete `Option_<T>` impl owner.

The `wave/prelude-text2` follow-up retired 8 of the text 10. The old free
calls fail with E0100 and a receiver hint:

| Removed text free | Replacement |
|-------------------|-------------|
| `byte_at(s, i)` | `s.byte_at(i)` |
| `slice(s, start, end)` | `s.slice(start, end)` |
| `substring(s, start, end)` | `s.slice(start, end)` (same `string.slice` CoreOp) |
| `to_string(x)` | `x.to_string()` |
| `starts_with(s, prefix)` | `s.starts_with(prefix)` |
| `ends_with(s, suffix)` | `s.ends_with(suffix)` |
| `contains(s, sub)` | `s.contains(sub)` |
| `index_of(s, sub)` | `s.index_of(sub)` |
| `clone(x)` | `x.clone()` |

The `std::collections::string` module-scoped frees for these eight names and
`clone` were deleted as well, so there is no second free spelling. `clone`
keeps a private generic identity shim in `std/prelude.ark` for receivers
without a `Clone` impl; a bare call and `use std::prelude::{clone}` are E0100.
`Vec<T>::clone` is shallow (shared element storage) until the pin refresh
allows a `T: Clone`-bounded deep impl.

The following remain `associated-or-method` and are **not** covered by the
exception:

- `Box_new` / `unbox`: the ownership checker special-cases the free names
  (ADR-088 §6), and the generic `Box<T>` associated-function / method spelling
  cannot infer `T` under the current lowering. The method form needs an
  ownership / MIR change.
- `eq`: `String`'s `==` lowers through the `eq` operator method; the bootstrap
  flat overlay does not ship `std::core::cmp` (`impl Eq for String`), so removing
  the free makes the pin lower String `==` as `i32.eq` (reference comparison)
  and produce invalid core Wasm. Keep the free until the overlay carries the
  `Eq` dispatch or the operator lowering has a string-eq fallback.

## Stable-64 deprecation wave (ADR-087, 2026-10-26)

The [#917](https://github.com/wogikaze/arukellt/issues/917) wave converted the
`stable` `must delete` families listed by [ADR-087](../adr/ADR-087-0x-deprecation-exception.md)
to `stability = "deprecated"` with `deprecated_by`, `deprecated_since = "0.1.0"`,
and `remove_in = "1.0.0"` (60 entries). The one deferral, `i8_to_i32`, was
converted on 2026-10-28 after `i8::to_i32` landed, so the numeric family is now
17 deprecated entries. The bare names emit **W0009** while they remain
resolvable; the gated prelude names also need an explicit
`use std::prelude::{name}` before the warning can fire. `remove_in` is an upper
bound under ADR-087, not the earliest removal. Wave 2 (2026-10-27) deleted the
31 seq/combinator entries from that set; the numeric, `vec` intrinsic-compat,
and assertion families remain deprecated for their own waves. The `i64` /
`f64` / `String` iterator consumer lanes listed as *planned* below are still
not callable for every row, so the removal table names the reachable loop or
`v.sort()` / `v.sort_by(cmp)` replacement alongside the target method shape.

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

### Numeric conversions (17 deprecated)

| Deprecated API | Replacement |
|----------------|-------------|
| `u8_to_i32(x)` | `(x).to_i32()` |
| `u16_to_i32(x)` | `(x).to_i32()` |
| `i8_to_i32(x)` | `(x).to_i32()` |
| `i16_to_i32(x)` | `(x).to_i32()` |
| `i32_to_u8(x)` | `(x).to_u8_wrapping()` |
| `i32_to_u16(x)` | `(x).to_u16_wrapping()` |
| `i32_to_i8(x)` | `(x).to_i8_wrapping()` |
| `i32_to_i16(x)` | `(x).to_i16_wrapping()` |
| `u32_to_u64(x)` | `(x).to_u64()` |
| `u64_to_u32(x)` | `(x).to_u32_wrapping()` |
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

The replacement is the `std::test::Test` associated surface; import it with
`use std::test::Test`.

| Deprecated API | Replacement |
|----------------|-------------|
| `assert_eq(a, b)` | `Test::assert_eq_i32(a, b)` |
| `assert_ne(a, b)` | `Test::assert_ne_i32(a, b)` |
| `assert_eq_i64(a, b)` | `Test::assert_eq_i64(a, b)` |
| `assert_eq_str(a, b)` | `Test::assert_eq_string(a, b)` |

### std::test assertion / expectation frees (18) — deprecated 2026-10-29

The [#917](https://github.com/wogikaze/arukellt/issues/917) `wave/test-group`
wave moved the `std::test` helpers onto the `Test` namespace. The 17 `stable`
entries are `deprecated` with `deprecated_by = "Test::<name>"`,
`deprecated_since = "0.1.0"`, and `remove_in = "1.0.0"`; the `experimental`
`bench` retires directly under ADR-014 D4. Wave 1 keeps every free callable
with **W0009**; Wave 2 deletes the declarations and the qualified / bare old
spellings then fail with **E0100**.

Import `use std::test::Test` and prefix the call with `Test::`:

| Deprecated API | Replacement |
|----------------|-------------|
| `assert_true(cond)` | `Test::assert_true(cond)` |
| `assert_false(cond)` | `Test::assert_false(cond)` |
| `assert_eq_debug(actual, expected)` | `Test::assert_eq_debug(actual, expected)` |
| `assert_eq_i32(actual, expected)` | `Test::assert_eq_i32(actual, expected)` |
| `assert_eq_i64(actual, expected)` | `Test::assert_eq_i64(actual, expected)` |
| `assert_eq_f64(actual, expected)` | `Test::assert_eq_f64(actual, expected)` |
| `assert_eq_string(actual, expected)` | `Test::assert_eq_string(actual, expected)` |
| `assert_eq_bool(actual, expected)` | `Test::assert_eq_bool(actual, expected)` |
| `assert_ne_i32(actual, unexpected)` | `Test::assert_ne_i32(actual, unexpected)` |
| `assert_ne_string(actual, unexpected)` | `Test::assert_ne_string(actual, unexpected)` |
| `expect_ok_i32(result)` | `Test::expect_ok_i32(result)` |
| `expect_err_string(result)` | `Test::expect_err_string(result)` |
| `expect_some_i32(option)` | `Test::expect_some_i32(option)` |
| `expect_none_i32(option)` | `Test::expect_none_i32(option)` |
| `assert_contains(haystack, needle)` | `Test::assert_contains(haystack, needle)` |
| `assert_eq_snapshot(actual, expected)` | `Test::assert_eq_snapshot(actual, expected)` |
| `assert_msg(cond, message)` | `Test::assert_msg(cond, message)` |
| `bench(name, iterations, f)` | `Test::bench(name, iterations, f)` (`experimental`; direct retirement) |

Wave 2 (2026-10-29) deleted the 17 deprecated declarations and the
`experimental` `bench` declaration together with their manifest entries. A
qualified `test::assert_eq_i32(...)` call now fails with **E0100** and a
`resolver::removed_names` hint that names the `Test` replacement; a bare
`assert_eq_i32(...)` gets the same hint from the bare-name table. The
`Test` associated functions are unaffected.

### Removed in Wave 2: monomorphic seq/combinator frees (31)

The 31 callable `stable` entries were deprecated in Wave 1 and deleted on
2026-10-27 under ADR-087 D2/D3 (#703 / #917). Bare calls now fail with
**E0100** and a `resolver::removed_names` hint; `filter_i32` remains
provisional (individual deferral) and is not part of this removal.

The `i32` producer/consumer lanes and the `Vec<String>` `map` / `filter` /
`any` / `find` lanes are verified on both targets. The `i64` / `f64` numeric
consumers and the `Vec<String>` reverse/contains lanes still use the
**reachable** explicit loop; the `i32` / `i64` / `String` sort lanes moved to
the verified `Vec<T: Ord>` receiver (`v.sort()`), and `f64` uses
`v.sort_by(total_cmp)`. `v.sort()` is correct for `Vec<i64>` and
`Vec<String>` at 5+ elements on both targets, including the gc core/test
path, so `seq::sort_by` was retired (#965).

| Removed API | Replacement |
|-------------|-------------|
| `sort_i32(v)` | `v.sort()` |
| `sort_i64(v)` | `v.sort()` |
| `sort_f64(v)` | `v.sort_by(total_cmp)` (`f64` is not `Ord`) |
| `sort_String(v)` | `v.sort()` |
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
values.sort()
```

---

## Provisional `std::seq` generic algorithms (9 removed 2026-10-29, #917)

The 9 provisional generic `std::seq` frees were removed directly under
ADR-014's provisional notice rule (changelog + this migration entry; no
ADR-087 stable window and no W0009). Import the receiver surface with
`use std::collections::vec`; old qualified calls fail with E0100 and a
`resolver::removed_names` hint.

| Removed API | Replacement |
|-------------|-------------|
| `seq::sort_by(v)` | `v.sort()` (`T: Ord`; `v.sort_by(cmp)` for a custom order) |
| `seq::binary_search_by(v, target)` | `v.binary_search(target)` |
| `seq::binary_search_by_key(v, key, target_key)` | `v.binary_search_by_key(target_key, key)` |
| `seq::lower_bound(v, target)` | `v.lower_bound(target)` |
| `seq::upper_bound(v, target)` | `v.upper_bound(target)` |
| `seq::min_by(v)` | `v.min()` |
| `seq::max_by(v)` | `v.max()` |
| `seq::min_by_key(v, key)` | `v.min_by_key(key)` |
| `seq::max_by_key(v, key)` | `v.max_by_key(key)` |

`min` / `max` / `min_by_key` / `max_by_key` trap on an empty vector, matching
the removed free functions. The 13 deprecated i32 wrappers (`binary_search`,
`min_i32`, `max_i32`, ...) remain callable until their own #703 Wave 2 and now
delegate to these methods.

`seq::sort_by` was removed with this wave once the `Vec<T: Ord>` method
scratch layout and the parametric `Ord::cmp` dispatch were fixed (#965).
`v.sort()` is correct for `i32`, `i64`, `String`, and custom-struct elements
at 5+ elements on both targets, including the gc core/test path.

## Currently Deprecated APIs

The following public entries currently carry `deprecated_by` in
`std/manifest.toml`; they are shown with ~~strikethrough~~ in the
[API reference](reference.md).

| Deprecated API | Replacement |
|----------------|-------------|
| `char_at` | `byte_at` |
| `concat` | `a.concat(b)` |
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
| `std::seq::binary_search` | `v.binary_search(target)` |
| `std::seq::min_i32` | `v.min()` |
| `std::seq::max_i32` | `v.max()` |
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
| `std::io::writer_write` | `Stdout::new().write(data)` / `Stderr::new().write(data)` from `use std::io`; memory sinks keep `w.write(data)` on `Writer` |
| `std::io::writer_write_str` | `Stdout::new().write_string(s)` / `Stderr::new().write_string(s)` from `use std::io`; memory sinks keep `w.write_string(s)` on `Writer` |

The two `std::io` rows are deleted: `std::io` now exposes the typed `Stdout` /
`Stderr` handles that implement `Write`, and the compiler has a raw stderr
write intrinsic that never appends a newline. A qualified old
call fails resolution with an **E0100** hint that names the typed-handle
replacement ([#693](https://github.com/wogikaze/arukellt/issues/693)). Every
other legacy `std::io` free function, including `reader_from_bytes`, the seven
`buf_*` entry points, `writer_new`, `write_all`, `write_string`,
`writer_flush`, and `writer_to_bytes`, was likewise removed with an E0100
replacement hint; see
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
| `canonical::canonical_scratch_new()` | `Component::scratch_new()` | unregistered source free; associated function |
| `canonical::canonical_lower_string(scratch, s)` | `Component::lower_string(scratch, s)` | unregistered source free; associated function |
| `canonical::canonical_lift_string(scratch, slice)` | `Component::lift_string(scratch, slice)` | unregistered source free; associated function |
| `canonical::canonical_lift_string_at(scratch, ptr, len)` | `Component::lift_string_at(scratch, ptr, len)` | unregistered source free; associated function |
| `canonical::canonical_lower_list_i32(scratch, items)` | `Component::lower_list_i32(scratch, items)` | unregistered source free; associated function |
| `canonical::canonical_lift_list_i32(scratch, slice)` | `Component::lift_list_i32(scratch, slice)` | unregistered source free; associated function |
| `canonical::canonical_lift_list_i32_at(scratch, ptr, count)` | `Component::lift_list_i32_at(scratch, ptr, count)` | unregistered source free; associated function |

Both targets are exercised by `stdlib_wit/wit_ast_parse.ark` +
`wit_ast_parse_wasm32.ark`, `stdlib_component/component_helpers.ark` +
`component_helpers_wasm32.ark`, `stdlib_component/handle_basic.ark` +
`handle_basic_wasm32.ark`, and `stdlib_component/canonical_string.ark` /
`canonical_string_wasm32.ark` plus `canonical_list.ark` /
`canonical_list_wasm32.ark` with `.expected` goldens.

`std::component::canonical` is now an E0100 anchor only: the 7 `canonical_*`
frees moved to `Component` associated functions, and `CanonicalSlice` lives in
`std::component`. The earlier report that an associated/method form returning
`Result<String, error::Error>` produced empty/NUL output on both targets was a
probe-side scratch index error (the probe wrote bytes at scratch offset
`base + i` while the lift read slot `ptr - 256 + 1 + i`), not a lowering
defect: the `Component` surface lifts `hi` correctly on wasm32-gc and wasm32
linear.

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
wave first moved the 132 operations whose emitter lowering was already valid
on both `wasm32-gc` and `--target wasm32` to one namespace type per lane
module, then the SIMD lowering fix moved the remaining 130. Every lane
operation is now an associated function; no top-level free function remains.
The old qualified call shape maps directly:

```ark
// before
use std::simd::f32x4
let v = f32x4::add(f32x4::splat(1.0f32), f32x4::splat(2.0f32))

// after
use std::simd::f32x4::F32x4
let v = F32x4::add(F32x4::splat(1.0f32), F32x4::splat(2.0f32))
```

The `[[functions]]` entries are gone and their `[[types]]` method lists are
the manifest surface. Because the entries were `experimental`, ADR-014 D4
allowed direct deletion; no W0009 deprecation window was opened and no
`deprecated_by` entry was added. A qualified old call such as
`f32x4::splat(...)` fails resolution with an **E0100** hint that names
`F32x4::splat(...)` and the import path.

| Module | Namespace type | Associated functions |
|--------|----------------|----------------------|
| `std::simd::f32x4` | `F32x4` | all lane operations |
| `std::simd::f64x2` | `F64x2` | all lane operations |
| `std::simd::i8x16` | `I8x16` | all lane operations |
| `std::simd::i16x8` | `I16x8` | all lane operations |
| `std::simd::i32x4` | `I32x4` | all lane operations |
| `std::simd::i64x2` | `I64x2` | all lane operations |
| `std::simd::u8x16` | `U8x16` | all lane operations |
| `std::simd::u16x8` | `U16x8` | all lane operations |
| `std::simd::u32x4` | `U32x4` | all lane operations |
| `std::simd::u64x2` | `U64x2` | all lane operations |
| `std::simd::v128` | `V128` | all lane operations |

### Lowering fixes that unblocked the final 130

The emitter fixes landed after the lane surface so that both canonical targets
lower every operation:

- The SIMD sub-opcode table had draft values for `i8x16`/`i16x8`/`i64x2`/
  `f32x4`/`f64x2` arithmetic and for the load/store-lane group. Corrected
  values make `f32x4::neg`, the `f64x2` arithmetic/min/max set, the saturating
  `i8x16`/`i16x8` ops, `i16x8::mul`, and the `i64x2` arithmetic/shift ops
  emit the intended instruction.
- The `i8x16`, `i16x8`, `i32x4`, and `i64x2` comparisons that were missing
  from the core-op registry and the 0xfd dispatch now have both (`le_s`/`le_u`/
  `ge_s`/`ge_u`, the full `i16x8` set, `i32x4::max_u`, `i64x2::gt_s`,
  `f64x2::min`/`max`).
- Unsigned lane intrinsics (`__simd_u8x16_*`, `u16x8`, `u32x4`, `u64x2`) are
  aliases of the corresponding signed machine ops where the bits agree, and
  have dedicated lowering for the unsigned-only compare/min/max forms.
- `i8x16::mul` has no Wasm instruction; the emitter multiplies the low and
  high halves as `i16x8` products, masks the low byte, and narrows.
- `u64x2::{lt_u, le_u, gt_u, ge_u}` have no Wasm instruction; the emitter
  flips each lane's sign bit and uses the signed `i64x2` compare.
- The i64x2 shift amount is an `i32` in Wasm; the MIR argument heuristic no
  longer promotes it to `i64` just because the intrinsic name contains `i64`.
- `v128::new` builds its byte lanes with `i8x16` replacement instead of the
  constant intrinsic, whose emitter only supported the all-zero vector.

The new `simd_conformance/u8x16_lane_ops`, `u16x8_lane_ops`,
`u32x4_lane_ops`, `u64x2_lane_ops`, `signed_lane_ops`, `float_lane_ops`, and
`v128_new` fixtures (each with a `_wasm32` twin) execute the full fixed surface
on both targets. The existing `stdlib_simd/simd_method_surface.ark` +
`_wasm32.ark` pair covers the original migrated surface, and
`diagnostics/removed_simd_free.ark` + `_wasm32.ark` pin the E0100 hint.

## std::bytes method wave (ADR-087, 2026-10-27)

The [#917](https://github.com/wogikaze/arukellt/issues/917) bytes wave landed
the receiver / associated surface for the shared `Vec<i32>` byte layout. The
eight unregistered helpers (`hex_char`, `hex_val_char`, `try_hex_val_char`, and
the five `try_read_*` readers) were retired immediately with E0100 hints
because they had no manifest lifecycle entry.

`Bytes`, `ByteBuf`, and `ByteCursor` are still `Vec<i32>` at runtime. Import
`use std::bytes` for the methods and `use std::bytes::Bytes` for the
receiver-less constructors/decoders.

### Removed in Wave 2 (stable)

Wave 1 made the 45 registered `stable` free functions `deprecated` with W0009,
`deprecated_since = "0.1.0"`, and `remove_in = "1.0.0"`. The Wave 2 deletion
removed the manifest entries and declarations, so qualified calls now fail
resolution with an **E0100** hint from `resolver::removed_names`. The
method/associated surface below is the only spelling.

| Removed | Replacement |
|---------|-------------|
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
| `bytes::hex_char` (removed in Wave 1) | `b.to_hex()` |
| `bytes::hex_val_char` (removed in Wave 1) | `Bytes::from_hex(s)` |
| `bytes::try_hex_val_char` (removed in Wave 1) | `Bytes::from_hex(s)` |
| `bytes::try_read_u8` (removed in Wave 1) | `c.try_read_u8()` |
| `bytes::try_read_u16_le` (removed in Wave 1) | `c.try_read_u16_le()` |
| `bytes::try_read_u32_le` (removed in Wave 1) | `c.try_read_u32_le()` |
| `bytes::try_read_u32_be` (removed in Wave 1) | `c.try_read_u32_be()` |
| `bytes::try_read_bytes` (removed in Wave 1) | `c.try_read_bytes(n)` |

The dual-target fixtures are `stdlib_bytes/bytes_method_surface*.ark`; the
E0100 goldens are `diagnostics/removed_bytes_free*.ark`.

## std::json / std::toml value-method wave (ADR-046, 2026-10-27)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and the
  experimental-entry rule in [ADR-014](../adr/ADR-014-stability.md) D4. Every
  removed manifest entry was `experimental`; qualified callers get **E0100**
  hints from `removed_json_member_hint` / `removed_toml_member_hint`.
- **Scorecard**: 728 → **701** public symbols; `associated-or-method`
  658 → **631**; manifest functions 507 → **489**; unregistered declarations
  222 → **213**.

| Before | After |
|--------|-------|
| `json::json_get(v, key)` | `v.get_field(key)` |
| `json::json_get_index(v, i)` | `v.get_index(i)` |
| `json::json_as_string(v)` | `v.as_string()` |
| `json::json_as_i32(v)` | `v.as_i32()` |
| `json::json_as_bool(v)` | `v.as_bool()` |
| `json::json_as_f64(v)` | `v.as_f64()` |
| `json::json_as_array(v)` | `v.as_array()` |
| `json::is_null(v)` / `json::is_bool(v)` / `json::is_number(v)` | `v.is_null()` / `v.is_bool()` / `v.is_number()` |
| `json::is_string(v)` / `json::is_array(v)` / `json::is_object(v)` | `v.is_string()` / `v.is_array()` / `v.is_object()` |
| `json::stringify(v)` | `v.to_text()` |
| `json::parse_error_message(e)` | `e.message()` |
| `toml::toml_get(v, key)` | `v.get_field(key)` |
| `toml::toml_table_keys(v)` | `v.table_keys()` |
| `toml::toml_as_string(v)` / `toml::toml_as_int(v)` / `toml::toml_as_bool(v)` | `v.as_string()` / `v.as_int()` / `v.as_bool()` |
| `toml::toml_as_float(v)` / `toml::toml_as_array(v)` / `toml::toml_as_table(v)` | `v.as_float()` / `v.as_array()` / `v.as_table()` |

### Associated-conversion wave (2026-10-27, #917 / wave/non-main-module)

- **Basis**: [ADR-046](../adr/ADR-046-free-function-eradication.md) D2 and the
  ADR-014 D4 experimental-entry rule. Every removed manifest entry was
  `experimental`; the unregistered declarations were never part of the stable
  64. Qualified and bare callers get **E0100** hints from
  `removed_json_member_hint` / `removed_toml_member_hint` /
  `removed_csv_member_hint` and the bare-name fallback in `removed_name_hint`.
- **Compiler unblock**: loaded-module bodies keep the source spelling
  `module::Type::method` because the resolver only canonicalizes the entry
  module. Lowering's signature-registry lookup missed the registered
  `Type::method` entry, so the call kept `fid=-1`; the fused in-loop emitter
  then emitted a permanent `drop; unreachable` instead of deferring, and the
  reachability worklist could drop the impl body. The fix canonicalizes the
  callee through `callee_spelling_variants` in lowering, the typechecker,
  edge discovery, the in-loop discovery oracle, and the drain-time name
  fallback. The impl and its mono instance were already in the fn table.
- **Removed**: 32 `std/manifest.toml` entries plus 25 unregistered
  declarations across `std/json.ark`, `std/json/parser.ark`,
  `std/toml.ark`, and `std/csv/mod.ark`. The text helpers are now private
  implementation details of the namespace types.
- **Fixtures**: `stdlib_json/json_associated_surface*.ark`,
  `stdlib_toml/toml_text_surface*.ark`,
  `stdlib_csv/csv_namespace_surface*.ark`, and the two-module regression
  `modules/non_main_associated_call/` (helper.ark calls the associated
  surface, main.ark asserts the output). Each pair compiles and runs on
  `wasm32-gc` (P2 component) and `--target wasm32` (linear).
- **Scorecard**: 588 → **531** public symbols; `associated-or-method`
  518 → **461**; manifest functions 417 → **385**; unregistered declarations
  172 → **147**. `must delete` stays 70 and `intrinsic-only` stays 1.

| Before | After |
|--------|-------|
| `json::quote_string(s)` / `json::json_escape(s)` | `json::JsonText::quote(s)` / `json::JsonText::escape(s)` |
| `json::skip_ws(s, pos)` / `json::parse_int_at(s, pos)` | `json::JsonText::skip_ws(s, pos)` / `json::JsonText::parse_int_at(s, pos)` |
| `json::parse_content_length(h)` / `json::find_key_pos(j, k, from)` | `json::JsonText::content_length(h)` / `json::JsonText::find_key_pos(j, k, from)` |
| `json::json_hex_byte(b)` / `json::json_hex_value(c)` | `json::JsonText::hex_byte(b)` / `json::JsonText::hex_value(c)` |
| `json::json_parse_string_at(s, p)` / `json::json_decode_escape_at(s, p)` | `json::JsonText::parse_string_at(s, p)` / `json::JsonText::decode_escape_at(s, p)` |
| `json::json_decode_escape(s, p)` / `json::json_decode_unicode_ascii(s, p)` | `json::JsonText::decode_escape(s, p)` / `json::JsonText::decode_unicode_ascii(s, p)` |
| `json::json_get_str(j, k)` / `json::json_get_str_after(j, k, f)` | `json::JsonText::get_str(j, k)` / `json::JsonText::get_str_after(j, k, f)` |
| `json::json_get_int(j, k)` | `json::JsonText::get_int(j, k)` |
| `json::json_str_result_ok(r)` / `_value(r)` / `_next(r)` | `r.is_ok()` / `r.value()` / `r.next()` on `JsonStrResult` |
| `json::JsonStrResult_new(...)` / `json::JsonEscapeResult_new(...)` | `json::JsonStrResult::new(...)` / `json::JsonEscapeResult::new(...)` |
| `json::json_stringify_i32(n)` / `_bool(b)` / `_string(s)` | `json::JsonValue::stringify_i32(n)` / `stringify_bool(b)` / `stringify_string(s)` |
| `json::json_null()` | `json::JsonValue::null_text()` |
| `json::json_parse_i32(s)` / `json::json_parse_bool(s)` | `json::JsonValue::parse_i32(s)` / `json::JsonValue::parse_bool(s)` |
| `json::stringify_pretty(v, indent)` | `json::JsonValue::pretty_text(v, indent)` |
| `json::json_field(k, v)` | `json::JsonField::new(k, v)` |
| `json::json_str(s)` / `json::json_i32(n)` / `json::json_bool(b)` | `json::JsonValue::string(s)` / `int(n)` / `bool(b)` |
| `json::json_null_val()` / `json::json_raw(s)` | `json::JsonValue::null()` / `json::JsonValue::raw(s)` |
| `json::json_obj_new()` / `json::json_arr_new()` | `Vec::new<json::JsonField>()` / `Vec::new<json::JsonValue>()` |
| `json::json_obj_string(f)` / `json::json_arr_string(i)` | `json::JsonValue::object(f).to_text()` / `json::JsonValue::array(i).to_text()` |
| `json::json_encode_string(s)` | `json::JsonValue::encode_string(s)` |
| `toml::toml_parse_line(line)` | `toml::TomlText::parse_line(line)` |
| `toml::toml_find_section(c, s)` / `toml::find_toml_section(c, s)` | `toml::TomlText::find_section(c, s)` |
| `toml::toml_find_value(c, k)` / `toml::find_toml_value(c, k)` | `toml::TomlText::find_value(c, k)` |
| `toml::toml_find_raw_value(c, k)` | `toml::TomlText::find_raw_value(c, k)` |
| `toml::toml_parse_string_array(raw)` | `toml::TomlText::parse_string_array(raw)` |
| `csv::csv_split_line(l)` / `csv::csv_parse_row(l)` | `csv::Csv::split_line(l)` / `csv::Csv::parse_row(l)` |
| `csv::csv_stringify_row(f)` / `csv::csv_stringify(rows)` | `csv::Csv::stringify_row(f)` / `csv::Csv::stringify(rows)` |
| `csv::csv_count_rows(s)` / `csv::csv_get_row_raw(s, i)` | `csv::Csv::count_rows(s)` / `csv::Csv::get_row_raw(s, i)` |
| `csv::csv_parse_row_at(s, i)` / `csv::csv_parse(s)` | `csv::Csv::parse_row_at(s, i)` / `csv::Csv::parse(s)` |
| `csv::csv_parse_with_header(s)` | `csv::Csv::parse_with_header(s)` |

### Document entry-point wave (2026-10-27, #917 / wave/json-csv)

- **Removed**: 3 `std/manifest.toml` entries (`json::parse`,
  `toml::toml_parse`, `toml::toml_stringify`) and 8 unregistered parse-step
  helpers (`JsonParseStep_ok` / `_err`, `json_parse_step_value` / `_next`,
  `TomlValStep_ok` / `_err`, `TomlKeyStep_ok` / `_err`). Everything was
  `experimental` (ADR-014 D4) or outside the manifest, so no W0009 window
  applies; qualified and bare calls get E0100 hints from
  `removed_json_member_hint` / `removed_toml_member_hint`.
- **Replacement**: `JsonValue::parse(doc)` / `v.to_text()`,
  `TomlValue::parse(doc)` / `v.to_text()`, `JsonParseStep::ok` / `err` with
  `step.value()` / `step.next()`, `TomlValStep::ok` / `err`, and
  `TomlKeyStep::ok` / `err`.
- **Payload inference (retested)**: with the rebuilt s2 (which contains
  `3472c53af` plus the payload/condition inference fixes), an **unannotated**
  `match json::JsonValue::parse(doc)` / `match toml::TomlValue::parse(doc)`
  infers `Result::Ok` / `Result::Err` correctly and validates on both targets.
  The committed pinned bootstrap predates those fixes, so fixture parity
  records its compile failure as a non-fatal note and validates/runs the s2
  artifact; the source-facing surface is not gated on the pin.
- **i64 payload (diagnosed and fixed)**: `TomlValue::to_text` itself was not
  the defect. The failing shape was a direct `TomlValue::Integer(42)`
  construction: the unannotated literal reached `gc_struct_set` as `i32`
  while the resolved variant layout declared an `i64` field, and
  `wasm-tools validate` rejected the module (`expected i64, found i32` in the
  WAT `struct.set`). `emit_gc_variant_payload_fields` now widens an `i32`
  payload to `i64` when the resolved variant layout declares `i64`. Values
  produced by `TomlValue::parse` were already i64 and validated before and
  after the change.
- **Same-name private helper collision (diagnosed and avoided)**: using both
  `JsonValue::to_text` and `TomlValue::to_text` in one program resolved the
  calls correctly in WAT (`call $JsonValue::to_text` /
  `call $TomlValue::to_text`), but the TOML body called a bare private
  `stringify_value`, and the two parser modules both declared that name. The
  resolver bound the JSON implementation inside the TOML method, so a
  TomlValue was stringified through the JsonValue layout (blank output on the
  linear target, a `ref.cast` trap on wasm32-gc). The private parser helpers
  are now module-prefixed (`json_parse_document` / `json_stringify_value` /
  `json_stringify_array`, `toml_*` counterparts); no duplicate top-level
  helper name remains between the two modules. The two-module regression
  fixture covers the combined surface.
- **Fixtures**: `stdlib_json/json_associated_surface*.ark` and
  `stdlib_toml/toml_text_surface*.ark` now cover the unannotated parse match
  and integer `to_text`; `modules/non_main_associated_call/` calls both
  parsers from a helper module. Each pair compiles, validates, and runs on
  `wasm32-gc` (P2 component) and `--target wasm32` (linear).
- **Scorecard**: 358 -> **347** public symbols; `associated-or-method`
  288 -> **277**; manifest functions 325 -> **322**; unregistered declarations
  34 -> **26**. `must delete` 70 and `intrinsic-only` 1 unchanged.

| Before | After |
|--------|-------|
| `json::parse(doc)` | `json::JsonValue::parse(doc)` |
| `toml::toml_parse(doc)` | `toml::TomlValue::parse(doc)` |
| `toml::toml_stringify(v)` | `v.to_text()` |
| `json::JsonParseStep_ok(v, n)` / `json::JsonParseStep_err(e, n)` | `json::JsonParseStep::ok(v, n)` / `json::JsonParseStep::err(e, n)` |
| `json::json_parse_step_value(step)` / `json::json_parse_step_next(step)` | `step.value()` / `step.next()` |
| `toml::TomlValStep_ok(v, n)` / `toml::TomlValStep_err(e, n)` | `toml::TomlValStep::ok(v, n)` / `toml::TomlValStep::err(e, n)` |
| `toml::TomlKeyStep_ok(p, n)` / `toml::TomlKeyStep_err(e, n)` | `toml::TomlKeyStep::ok(p, n)` / `toml::TomlKeyStep::err(e, n)` |

### Residual blocks

- **`Vec<String>::concat`**: the method lowers to a `String::concat` receiver
  on wasm32-gc and fails validation for a `Vec<String>` local (pre-existing,
  independent of the associated-call fix). Join with an explicit loop or use
  `len` / `get_unchecked` in the meantime.
- **Bootstrap adapter**: the checked-in associated surface is now the
  bootstrap input. The flat overlay ships `std/toml.ark`, `std/host/fs.ark`,
  `std/host/clock.ark`, and `std/io/mod.ark`, so `toml::TomlText::*`,
  `Fs::*`, and `Clock::*` resolve directly and the old free-name adapters are
  gone. The only remaining `scripts/selfhost/checks.py` rewrite is for
  `Env::*`: the pin predates the #917 MIR env call-result typing, so a
  reachable `Env::args` would lower to invalid core Wasm. That adapter is
  removed when the pin includes the #917 typing.

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
| `io::file_from_fd` / `io::file_read` / `io::file_write` | `std::host::fs::File` (`File::open` + `Read` / `Seek`; `File::create` + `Write` / `flush`) |

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
- `Stdout::new()` / `Stderr::new()` are byte-faithful `Write` sinks:
  `write(Vec<i32>)` emits the byte image unchanged and returns its exact
  length, `write_string(s)` emits UTF-8 bytes, and `flush` is synchronous. In
  particular `Stderr::write` never appends a newline. The tagged
  `stdout()` / `stderr()` vectors remain only as `std::io` compatibility
  values with no write path of their own.
- `File::create(path)` starts an empty write image. `write` writes at the
  handle cursor (extending with zero bytes when the cursor is past the end),
  and `flush` / `close` write the whole image to the path. Other readers keep
  seeing the old bytes until that flush succeeds. `File::open` rejects
  `write`, `File::create` rejects `read`, and a read-only `flush` is a no-op.
  `close` on an untouched `File::create` truncates the file.

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

**Tagged output writers before / after:**

<!-- skip-doc-check reason="migration shape example; not fixture-backed" owner="#693" kind="non-runnable" expires="2026-12-31" -->
```ark
// before
let w: Vec<i32> = io::stdout()
io::writer_write_str(w, "text")?

// after
let out: io::Stdout = io::Stdout::new()
out.write_string("text")?
```

Evidence for the wave:

- `tests/fixtures/diagnostics/removed_io_tagged_writer_write.ark` and
  `tests/fixtures/diagnostics/removed_io_tagged_writer_write_str.ark` (E0100
  replacement hints for the deleted tagged writers).
- `tests/fixtures/stdlib_io_traits/typed_stdout_write.ark` and
  `tests/fixtures/stdlib_io_traits/typed_stderr_raw.ark` exercise the byte
  contract on both `wasm32-gc` and `wasm32`; the stderr fixture fails if the
  raw write appends a newline.
- `tests/fixtures/diagnostics/removed_io_free_function.ark` (E0100 replacement
  hint for a removed member).
- `tests/fixtures/diagnostics/removed_io_compat_free_function.ark` (E0100 hint
  for a wrapper retired in the Phase F completion wave).
- The migrated `tests/fixtures/stdlib_io*`, `tests/fixtures/stdlib_trait/`, and
  `tests/fixtures/question_mark/error_ecosystem.ark` fixtures compile and run
  on both `wasm32-gc` and `wasm32`.
- `use std::io` no longer emits the retired "moved to `std::host::stdio`"
  W0009 warning (ADR-085 D1); `std::io` is the canonical abstraction module.

## std::wasm builder namespace wave (ADR-046, 2026-10-27)

The `std::wasm` builder family was an `experimental` free-function module. The
wave moves 72 manifest entries and all 34 unregistered declarations to seven
namespace / value types. The qualified call shape changes from a module free
call to an associated function or receiver method:

<!-- skip-doc-check reason="migration shape example; not fixture-backed" owner="#917" kind="non-runnable" expires="2026-12-31" -->
```ark
// before
use std::wasm
let m = wasm::module_new()
let ft = wasm::functype_new(params, results)
let ti = wasm::module_add_type(m, ft)
let op = wasm::op_i32_add()

// after
use std::wasm
let m: wasm::WasmModuleBuilder = wasm::WasmModuleBuilder::new()
let ft: wasm::FuncType = wasm::FuncType::new(params, results)
let ti: i32 = m.add_type(ft)
let op: i32 = wasm::Opcode::i32_add()
```

Because every entry was `experimental`, ADR-014 D4 allows direct deletion; no
W0009 deprecation window was opened and no `deprecated_by` entry was added. A
qualified old call such as `wasm::module_new()` fails resolution with an
**E0100** hint that names the owning type.

| Family | Before | After |
|--------|--------|-------|
| Module header | `wasm_magic`, `wasm_version`, `module_encode_header` | `WasmModuleBuilder::magic()`, `::version()`, `::encode_header()` |
| Module builder | `module_new`, `module_add_type`, `module_add_func`, `module_add_memory`, `module_add_export_func`, `module_type_count`, `module_func_count`, `module_mem_count`, `module_export_count` | `WasmModuleBuilder::new()`, `m.add_type(ft)`, `m.add_func(type_idx)`, `m.add_memory(limits)`, `m.add_export_func(name, func_idx)`, `m.type_count()`, `m.func_count()`, `m.mem_count()`, `m.export_count()` |
| Function type | `functype_new`, `functype_param_count`, `functype_result_count`, `functype_get_param`, `functype_get_result` | `FuncType::new(params, results)`, `ft.param_count()`, `ft.result_count()`, `ft.get_param(idx)`, `ft.get_result(idx)` |
| Limits | `limits_unbounded`, `limits_bounded`, `limits_min`, `limits_has_max`, `limits_max` | `Limits::unbounded(min)`, `Limits::bounded(min, max)`, `l.min()`, `l.has_max()`, `l.max()` |
| Section IDs | `section_type`, `section_import`, `section_function`, `section_table`, `section_memory`, `section_global`, `section_export`, `section_start`, `section_element`, `section_code`, `section_data` | `Section::type_id()`, `::import_id()`, `::function_id()`, `::table_id()`, `::memory_id()`, `::global_id()`, `::export_id()`, `::start_id()`, `::element_id()`, `::code_id()`, `::data_id()` |
| Value types | `valtype_i32` … `valtype_externref`, `reftype_funcref`, `reftype_externref`, `numtype_i32` … `numtype_f64` | `ValType::i32()`, `i64()`, `f32()`, `f64()`, `v128()`, `funcref()`, `externref()` |
| Opcodes | `op_unreachable` … `op_i64_mul` (45 entries) | `Opcode::unreachable()`, `nop()`, `block()`, `loop_()`, `if_()`, `else_()`, `end()`, `br()`, `br_if()`, `return_()`, `call()`, `drop()`, `select()`, `local_get()`, `local_set()`, `local_tee()`, `global_get()`, `global_set()`, `i32_load()`, `i32_store()`, `memory_size()`, `memory_grow()`, `i32_const()`, `i64_const()`, `f32_const()`, `f64_const()`, `i32_eqz()`, `i32_eq()`, `i32_ne()`, `i32_lt_s()`, `i32_gt_s()`, `i32_le_s()`, `i32_ge_s()`, `i32_add()`, `i32_sub()`, `i32_mul()`, `i32_div_s()`, `i32_and()`, `i32_or()`, `i32_xor()`, `i32_shl()`, `i32_shr_s()`, `i64_add()`, `i64_sub()`, `i64_mul()` |
| LEB128 | `leb128_encode_u32`, `leb128_size_u32` | `Leb128::encode_u32(x)`, `Leb128::size_u32(x)` |
| v128 bitwise | `v128_and`, `v128_or`, `v128_xor`, `v128_not`, `v128_andnot`, `v128_bitselect`, `v128_any_true` | `V128::and`, `or`, `xor`, `not`, `andnot`, `bitselect`, `any_true` from `use std::simd::v128::V128` |
| v128 reinterpret (removed) | `v128_reinterpret_i8x16`, `_i16x8`, `_i32x4`, `_i64x2`, `_f32x4`, `_f64x2` | no replacement: v128 is one type in Ark, so the old identity helper is unnecessary; pass the value directly |
| v128 memory | `v128_load`, `v128_store`, `v128_load_splat`, `v128_load_lane`, `v128_store_lane` | `V128Memory::load(address, offset)`, `store(address, offset, value)`, `load_splat(address, offset)`, `load_lane(address, offset, value, lane)`, `store_lane(address, offset, value, lane)`, plus the widening loads `load8x8_s` / `load8x8_u` / `load16x4_s` / `load16x4_u` / `load32x2_s` / `load32x2_u` |

`Section::import_id` / `export_id` and `Opcode::loop_` / `if_` / `else_` /
`return_` carry the suffix because `import`, `export`, `loop`, `if`, `else`,
and `return` are hard keywords.

### v128 memory: `V128Memory` associated functions (5)

The 0xfd linear-memory instructions now lower, so the five experimental
`std::wasm` load/store frees moved onto the `V128Memory` namespace type:

| Before | After |
|--------|-------|
| `v128_load(ptr, offset)` | `V128Memory::load(address, offset)` |
| `v128_store(ptr, offset, value)` | `V128Memory::store(address, offset, value)` |
| `v128_load_splat(ptr, offset)` | `V128Memory::load_splat(address, offset)` |
| `v128_load_lane(ptr, offset, value, lane)` | `V128Memory::load_lane(address, offset, value, lane)` |
| `v128_store_lane(ptr, offset, value, lane)` | `V128Memory::store_lane(address, offset, value, lane)` |

`V128Memory` also exposes the six widening loads: `load8x8_s/u`,
`load16x4_s/u`, and `load32x2_s/u`. Every function takes a byte address and a
byte offset; the emitter adds the two at runtime and leaves the instruction's
memarg offset at zero. `load_splat` loads one byte and splats it across all 16
lanes (`v128.load8_splat`); `load_lane` / `store_lane` move one 32-bit lane, so
`lane` is `0..3`.

Because every removed entry was `experimental`, ADR-014 D4 allows direct
deletion; no W0009 window was opened. A qualified old call such as
`wasm::v128_load(ptr, offset)` fails resolution with an **E0100** hint that
names the `V128Memory` function.

### Evidence

- `tests/fixtures/stdlib_wasm/{wasm_basic,valtype_ext,opcode_basic,leb128_basic,module_builder}.ark`
  were migrated to the associated spelling; their `.expected` output is
  unchanged except `valtype_ext` (the `reftype_*` duplicates collapsed into
  `ValType`).
- `tests/fixtures/stdlib_wasm/wasm_method_surface.ark` + `_wasm32.ark` assert
  the whole builder surface and the `V128` bitwise replacement on both targets.
- `tests/fixtures/diagnostics/removed_wasm_free.ark` + `_wasm32.ark` pin the
  E0100 hint.
- `tests/fixtures/stdlib_wasm/v128_memory_surface.ark` + `_wasm32.ark` store and
  reload vectors through `V128Memory`, assert the lane values, and cover
  `load_splat`, `load_lane`, `store_lane`, and the widening loads on both
  targets.
- `tests/fixtures/diagnostics/removed_wasm_v128_memory_free.ark` +
  `_wasm32.ark` pin the E0100 hints for the five retired frees.

## Iterator v2 status

The current iterator implementation is intentionally narrower than the final
trait-first design. The following path is implemented and verified for both
`wasm32-gc` and `wasm32`:

```ark
use std::core::{Range, RangeInclusive}
use std::core::iter

fn main() {
    let values: Vec<i32> = RangeInclusive::new(4, 6).into_iter().collect()
    let collected: Vec<i32> = Vec::from_iter(Range::new(0, 3).into_iter())
    let count = Range::new(0, 3).into_iter().count()
    let total = Range::new(0, 3).into_iter().sum()
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
the range fixture verifies `for item in Range::new(2, 5)` and
`for item in RangeInclusive::new(3, 5).into_iter()` on both backends.
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

## std::path / std::time / std::random receiver wave (ADR-046, ADR-087, 2026-10-27)

The 15 `stable` free helpers in these three pure modules completed the
[ADR-087](ADR-087-0x-deprecation-exception.md) 0.x window as `deprecated`
entries (`deprecated_since = "0.1.0"`, `remove_in = "1.0.0"`) with live
**W0009** warnings, then were removed in the Wave 2 step. Qualified old calls
now fail name resolution with **E0100** plus the receiver replacement hint
from `resolver::removed_names`.

The canonical surface is a receiver type in each module: `std::path::Path`,
`std::time::Duration`, and `std::random::Rng`. The unregistered
`path::last_index_of` helper also became private; its E0100 hint points at the
`Path` methods (`file_name`, `extension`, `parent`, `stem`, `normalize`) and
`String::index_of`.

| Before | After |
|--------|-------|
| `path::join(base, child)` | `Path::new(base).join(child)` |
| `path::parent(path)` | `Path::new(path).parent()` |
| `path::file_name(path)` | `Path::new(path).file_name()` |
| `path::extension(path)` | `Path::new(path).extension()` |
| `path::with_extension(path, ext)` | `Path::new(path).with_extension(ext)` |
| `path::is_absolute(path)` | `Path::new(path).is_absolute()` |
| `path::stem(path)` | `Path::new(path).stem()` |
| `path::normalize(path)` | `Path::new(path).normalize()` |
| `path::components(path)` | `Path::new(path).components()` |
| `time::duration_ms(start, end)` | `Duration::between(start, end).as_ms()` |
| `time::duration_us(start, end)` | `Duration::between(start, end).as_us()` |
| `time::duration_ns(start, end)` | `Duration::between(start, end).as_ns()` |
| `random::seeded_random(seed)` | `Rng::new(seed).next()` |
| `random::seeded_range(seed, lo, hi)` | `Rng::new(seed).range(lo, hi)` |
| `random::shuffle_i32(vec, seed)` | `Rng::new(seed).shuffle(vec)` |

`Path`, `Duration`, and `Rng` are value wrappers over the former argument
shapes, and `Path::into_string()` returns the wrapped string. The
`stdlib_path/path_method_surface*`, `stdlib_time/duration_methods*`, and
`stdlib_random/rng_methods*` fixture pairs compile and execute the surface on
both `wasm32-gc` and `--target wasm32`; the moved W0009 goldens were replaced
by `diagnostics/removed_{path,time,random}_free*` E0100 goldens.

---

## std::core receiver wave (ADR-046, ADR-087, 2026-10-27)

The eight `stable` free helpers in `std::core` plus the unregistered
`identity` declaration completed the
[ADR-087](ADR-087-0x-deprecation-exception.md) 0.x window as `deprecated`
entries (`deprecated_since = "0.1.0"`, `remove_in = "1.0.0"`) with live
**W0009** qualified-call warnings, then were removed in the Wave 2 step.
Qualified old calls now fail name resolution with **E0100** plus the receiver
replacement hint from `resolver::removed_names`.

The canonical surface is the inherent / associated form:
`Range::new(start, end)`, `r.contains(value)`, `r.len()`,
`RangeInclusive::new(start, end)`, `r.contains(value)`, `a.cmp(b)` from the
`Ord` trait, `e.message()`, and `h1.combine_hash(h2)`. `Range::new` stays
bound to the `core.range_new` CoreOp, so the GC struct constructor and the
linear-memory 8-byte record path keep their existing lowering. Receiver
methods on an enum-variant struct literal (`Error::IndexOutOfBounds { .. }`)
resolve against the owning enum while the payload layout stays in place for
emit, so the method call lowers and runs on both targets.

| Before | After |
|--------|-------|
| `core::range_new(start, end)` | `Range::new(start, end)` from `use std::core::{Range}` |
| `core::range_contains(r, value)` | `r.contains(value)` |
| `core::range_len(r)` | `r.len()` |
| `core::range_inclusive_new(start, end)` | `RangeInclusive::new(start, end)` from `use std::core::{RangeInclusive}` |
| `core::range_inclusive_contains(r, value)` | `r.contains(value)` |
| `core::cmp_i32(a, b)` | `a.cmp(b)` from `use std::core::cmp` |
| `error::error_message(e)` | `e.message()` |
| `hash::combine(h1, h2)` | `h1.combine_hash(h2)` |
| `core::identity(x)` | removed; delete the call (no replacement) |

The `stdlib_core/core_method_surface*` fixture pair compiles, validates, and
executes the surface on both `wasm32-gc` and `--target wasm32`; the W0009
goldens were replaced by `diagnostics/removed_core_free*` E0100 goldens.

## std::core raw namespace wave (ADR-046, 2026-10-27)

The sealed `std::core::raw` module no longer exports free functions. Its 20
representation bridges are now associated functions on `RawArray`,
`RawString`, and `RawFloat`; the canonical `raw_*` spellings remain private
CoreOp-bound helpers so the GC / linear-memory lowering table in
`data/core-ops.toml` is unchanged. User code cannot import the sealed module
(RFC-006), so there is no user migration; copied bare spellings fail with
**E0100** and a hint that names the public replacement.

| Old sealed bridge | Namespace spelling | Public replacement |
|-------------------|--------------------|--------------------|
| `raw_array_new(cap)` | `RawArray::new<T>(cap)` | `Vec::new<T>()` / `Vec::with_capacity<T>(cap)` |
| `raw_array_len(v)` | `RawArray::len(v)` | `v.len()` |
| `raw_array_get_unchecked(v, i)` | `RawArray::get_unchecked(v, i)` | `v.get_unchecked(i)` via `use std::collections::vec` |
| `raw_array_set_unchecked(v, i, x)` | `RawArray::set_unchecked(v, i, x)` | `v.set(i, x)` via `use std::collections::vec` |
| `raw_array_grow(v, n)` | `RawArray::grow(v, n)` | stdlib-only; no public replacement |
| `raw_array_set_len(v, n)` | `RawArray::set_len(v, n)` | stdlib-only; no public replacement |
| `raw_string_new()` | `RawString::new()` | `String_new()` |
| `raw_string_from_bytes(b)` | `RawString::from_bytes(b)` | `String::from_utf8_lossy(b)` for validated UTF-8 input |
| `raw_string_len_bytes(s)` | `RawString::len_bytes(s)` | `s.len_bytes()` |
| `raw_string_byte_at_unchecked(s, i)` | `RawString::byte_at_unchecked(s, i)` | `s.byte_at(i)` |
| `raw_string_clone(s)` | `RawString::clone(s)` | `s.clone()` |
| `raw_string_push_char(s, c)` | `RawString::push_char(s, c)` | `push_char(s, c)` |
| `raw_parse_i32(s)` | `RawString::parse_i32(s)` | `i32::try_from(s)` |
| `raw_parse_i64(s)` | `RawString::parse_i64(s)` | `i64::try_from(s)` |
| `raw_parse_f64(s)` | `RawString::parse_f64(s)` | `f64::try_from(s)` |
| `raw_f64_sqrt(x)` | `RawFloat::sqrt(x)` | `x.sqrt()` via `use std::core::convert` |
| `raw_f64_bits_lo(x)` | `RawFloat::bits_lo(x)` | `x.to_bits_lo()` |
| `raw_f64_bits_hi(x)` | `RawFloat::bits_hi(x)` | `x.to_bits_hi()` |
| `raw_f64_to_string(x)` | `RawFloat::to_string(x)` | `x.to_string()` |
| `raw_f32_promote_f64(x)` | `RawFloat::promote_f32(x)` | `f32_to_f64(x)` / `x.to_f64()` |

The `stdlib_core/raw_consumer_surface*` pair compiles, validates, and runs on
`wasm32-gc` and `--target wasm32`. The sealed module's own test module passes on
`--target wasm32` (array, string, and scalar suites) and compiles/validates on
`wasm32-gc`; the gc test wrapper does not package the P2 core module as a
component, so that direct test execution is a harness residual. Production
call sites bind the CoreOp and the consumer fixtures pass on both targets.

## std::host::fs `Fs` namespace wave (ADR-046, ADR-087, 2026-10-27)

The whole-path host filesystem helpers moved from free functions to the `Fs`
namespace type. `File` keeps the image-handle / `Read` / `Write` / `Seek`
surface; `Fs` owns the one-shot reads, writes, probes, and the honest
unsupported `read_dir` / `metadata` rejections.

Import the namespace explicitly:

```ark
use std::host::fs::Fs

let text = Fs::read_to_string("data.txt")
match text {
    Ok(contents) => println(contents),
    Err(message) => eprintln(message),
}
```

### Removed now: provisional / experimental / deprecated frees (11)

These entries were non-stable (or already-deprecated aliases), so ADR-046 D4
allowed removal with a migration note. Qualified old calls fail with **E0100**
and the `resolver::removed_names` hint.

| Old API | Replacement |
|---------|-------------|
| `std::host::fs::read_to_string(path)` | `Fs::read_to_string(path)` |
| `std::host::fs::read_bytes(path)` | `Fs::read_bytes(path)` |
| `std::host::fs::write_string(path, contents)` | `Fs::write_string(path, contents)` |
| `std::host::fs::write_bytes(path, bytes)` | `Fs::write_bytes(path, bytes)` |
| `std::host::fs::is_file(path)` | `Fs::is_file(path)` |
| `std::host::fs::is_dir(path)` | `Fs::is_dir(path)` |
| `std::host::fs::exists(path)` | `Fs::is_readable_file(path)` (read probe, not path existence) |
| `std::fs::read_dir(path)` | `Fs::read_dir(path)` (returns the unsupported-capability error) |
| `std::fs::metadata(path)` | `Fs::metadata(path)` (returns the unsupported-capability error) |
| `std::fs::is_file(path)` | `Fs::is_file(path)` |
| `std::fs::is_dir(path)` | `Fs::is_dir(path)` |

`std::fs` remains as a deprecated facade that re-exports `std::host::fs::Fs`;
the module-level W0009 hint points at `std::host::fs`.

### Removed in Wave 2: stable wrappers (5)

The ADR-087 W0009 window closed with the Wave 2 change (2026-10-28): the five
`stable` entries and their declarations are gone, and a qualified old call
fails with **E0100** plus the `resolver::removed_names` hint.

| Old API | Replacement |
|---------|-------------|
| `std::host::fs::is_readable_file(path)` | `Fs::is_readable_file(path)` |
| `std::fs::read_string(path)` | `Fs::read_to_string(path)` |
| `std::fs::write_string(path, contents)` | `Fs::write_string(path, contents)` |
| `std::fs::exists(path)` | `Fs::is_readable_file(path)` |
| `std::fs::is_readable_file(path)` | `Fs::is_readable_file(path)` |

The E0100 goldens are `diagnostics/removed_host_fs_stable_free*`.

### Retained: experimental fd helpers (3)

`fd_seek`, `fd_tell`, and `fd_fdstat_errno` stay experimental free functions.
They are `wasm32` / WASI P1 only (`t3 = false`), so they cannot be exercised on
`wasm32-gc`; moving them to `Fs` without a target-parity fixture would claim a
surface that does not lower on the primary target. They are the exact exception
list of [ADR-092](../adr/ADR-092-host-core-namespace-free-exception.md) §4 and
remain a reported residual after the host/core wave.

## std::host::clock `Clock` namespace wave (ADR-046, ADR-087, 2026-10-27)

The two `stable` host clock reads moved from module free functions to the
`Clock` namespace type. `Clock` owns the monotonic and wall-clock reads; the
WASI clock intrinsics stay behind its impl.

Import the namespace explicitly:

```ark
use std::host::clock::Clock

let started = Clock::monotonic_now()
let wall = Clock::now_ms()
```

### Removed in Wave 2: stable clock frees (2)

The ADR-087 W0009 window closed with the Wave 2 change (2026-10-28): both
`stable` entries and their declarations are gone, and a qualified old call
fails with **E0100** plus the `resolver::removed_names` hint.

| Old API | Replacement |
|---------|-------------|
| `std::host::clock::monotonic_now()` | `Clock::monotonic_now()` |
| `std::host::clock::now_ms()` | `Clock::now_ms()` |

The hint covers the `clock::`, `host::clock::`, and `std::host::clock::`
qualified spellings. The `Clock` namespace itself is `provisional`; promoting
it to `stable` is separate from the free-function removal.

The in-tree compiler call sites use `Clock::` directly. The committed pinned
bootstrap predates loading the namespace source into its flat overlay, so
`scripts/selfhost/checks.py` rewrites the compiler's `Clock::` calls to the
free builtins in the generated overlay only. Checked-in source keeps the
namespace; delete the adapter when the pin is refreshed (ADR-029).

The `stdlib_host/clock_namespace_surface*` fixture pair compiles, validates,
and executes the two reads on both `wasm32-gc` and `--target wasm32`; the E0100
goldens are `diagnostics/removed_host_clock_free*`.

### Removed in Wave 2: `std::env::get_var` (1)

`std::env::get_var` was the last `deprecated module frees` entry. Its ADR-087
window is closed, so the manifest entry and both declarations (`std::env` and
the duplicate `std::host::env` alias) were deleted on 2026-10-28. A qualified
old call fails with **E0100** (`undefined module member: env::get_var`).

| Old API | Replacement |
|---------|-------------|
| `std::env::get_var(name)` | `Env::var(name)` from `use std::host::env::Env` |
| `std::host::env::get_var(name)` | `Env::var(name)` from `use std::host::env::Env` |

The `Env` namespace is `provisional`. `Env::args`, `Env::var`,
`Env::arg_count`, `Env::arg_at`, `Env::var_or_default`, and `Env::has_flag`
compile, validate, and run on both `wasm32-gc` (P2 component) and
`--target wasm32` (linear); the `stdlib_host/env_args_surface*` and
`stdlib_host/env_namespace_surface*` fixture pairs pin them.

### Removed in Wave 2: `std::host::env` and `std::env` free functions (11)

The remaining env free functions and the legacy `std::env` module path were
deleted on 2026-10-28. `std::host::env` is now the `Env` namespace only, and
`std::env` no longer resolves as a module. Qualified old calls fail with
**E0100**. The `args()` prelude binding (`std::prelude::args`) is a separate
prelude entry outside the v2 auto-imported stable set, so it requires
`use std::prelude::args`; it is unchanged by this wave.

| Old API | Replacement |
|---------|-------------|
| `env::args()` / `std::env::args()` | `Env::args()` |
| `env::var(name)` / `std::env::var(name)` | `Env::var(name)` |
| `env::arg_count()` / `std::env::arg_count()` | `Env::arg_count()` |
| `env::arg_at(index)` / `std::env::arg_at(index)` | `Env::arg_at(index)` |
| `env::var_or_default(name, default)` / `std::env::var_or_default(...)` | `Env::var_or_default(name, default)` |
| `env::has_flag(flag)` | `Env::has_flag(flag)` |

Import `use std::host::env::Env` to reach them. `Env::arg_at` calls the
`__intrinsic_arg_at` intrinsic directly: the `wasm32-gc` P2 path indexes the
canonical `get-arguments` list and the linear path indexes the Preview 1 argv
table, so both targets return the argument at `index`.

### MIR call-result typing for the env intrinsics

The blocker recorded in the previous wave was fixed in the MIR call-result
typing (`src/compiler/mir/lower/call_type_fallback_stdlib.ark`):
`__intrinsic_args` types its destination as `vec:String` and
`__runtime_abi_env_var` / `__intrinsic_arg_at` as `Option_String`, the same
treatment the fs file intrinsics already had. The temporary `Option<String>`
bindings in `Env::var` / `Env::arg_at` are gone, and `Env::args` is a plain
`__intrinsic_args()` call that validates on both targets.

## std::collections::compiler type surface (ADR-046, 2026-10-29)

The 14 `std::collections::compiler` free functions were removed by #917 in the
ADR-014 `experimental` window. The replacement is the `Arena` / `SlotMap` /
`Interner` type surface; import the type and call the associated constructor
or receiver method.

| Removed free | Replacement |
|--------------|-------------|
| `compiler::arena_new()` | `Arena::new()` |
| `compiler::arena_alloc(a, value)` | `a.alloc(value)` |
| `compiler::arena_get(a, idx)` | `a.get(idx)` |
| `compiler::arena_len(a)` | `a.len()` |
| `compiler::slotmap_new()` | `SlotMap::new()` |
| `compiler::slotmap_insert(m, value)` | `m.insert(value)` |
| `compiler::slotmap_get(m, key)` | `m.get(key)` |
| `compiler::slotmap_remove(m, key)` | `m.remove(key)` |
| `compiler::slotmap_contains(m, key)` | `m.contains(key)` |
| `compiler::slotmap_len(m)` | `m.len()` |
| `compiler::interner_new()` | `Interner::new()` |
| `compiler::interner_intern(i, value)` | `i.intern(value)` |
| `compiler::interner_lookup(i, sym)` | `i.lookup(sym)` |
| `compiler::interner_len(i)` | `i.len()` |

```ark
// Before
use std::collections::compiler
let mut a: Vec<i32> = compiler::arena_new()
let id = compiler::arena_alloc(a, 42)

// After
use std::collections::compiler::Arena
let mut a: Arena = Arena::new()
let id = a.alloc(42)
```

Qualified old calls fail with **E0100** and the
`resolver::removed_names::removed_compiler_member_hint` replacement hint.

## std::collections bimap / trie type surface (ADR-046, 2026-10-29)

The 10 `bimap_i32_i32_*` and 7 `trie_*` free functions were unregistered
top-level declarations, so #917 removed them without an ADR-014 deprecation
window and without W0009 warnings. Import `Bimap` or `Trie` and use the
associated constructor plus receiver methods.

| Removed free | Replacement |
|--------------|-------------|
| `bimap::bimap_i32_i32_new()` | `Bimap::new()` |
| `bimap::bimap_i32_i32_insert(m, k, v)` | `m.insert(k, v)` |
| `bimap::bimap_i32_i32_get_by_key(m, k)` | `m.get_by_key(k)` (`Option<i32>`) |
| `bimap::bimap_i32_i32_get_by_value(m, v)` | `m.get_by_value(v)` (`Option<i32>`) |
| `bimap::bimap_i32_i32_contains_key(m, k)` | `m.contains_key(k)` |
| `bimap::bimap_i32_i32_contains_value(m, v)` | `m.contains_value(v)` |
| `bimap::bimap_i32_i32_remove_by_key(m, k)` | `m.remove_by_key(k)` |
| `bimap::bimap_i32_i32_remove_by_value(m, v)` | `m.remove_by_value(v)` |
| `bimap::bimap_i32_i32_len(m)` | `m.len()` |
| `bimap::bimap_i32_i32_is_empty(m)` | `m.is_empty()` |
| `trie::trie_new()` | `Trie::new()` |
| `trie::trie_insert(t, k, v)` | `t.insert(k, v)` |
| `trie::trie_get(t, k)` | `t.get(k)` (`Option<i32>`) |
| `trie::trie_contains(t, k)` | `t.contains(k)` |
| `trie::trie_has_prefix(t, p)` | `t.has_prefix(p)` |
| `trie::trie_remove(t, k)` | `t.remove(k)` |
| `trie::trie_count_with_prefix(t, p)` | `t.count_with_prefix(p)` |

```ark
// Before
use std::collections::bimap
let mut bm = bimap::bimap_i32_i32_new()
bimap::bimap_i32_i32_insert(bm, 1, 100)

// After
use std::collections::bimap::Bimap
let mut bm = Bimap::new()
bm.insert(1, 100)
```

`Bimap` is monomorphic over `i32` keys and `i32` values; `Trie` is
monomorphic over `String` keys and `i32` values. The backing storage is a flat
`Vec<i32>` (the bimap pair list and the trie's 256-way node pool), so generic
`Bimap<K, V>` / `Trie<V>` spellings would need a different runtime
representation and are deliberately not part of this surface. `Trie` has no
`len`: the node pool does not track stored-key count, and
`count_with_prefix("")` is the O(n) total count.

Qualified old calls fail with **E0100** and the
`resolver::removed_names::removed_bimap_member_hint` /
`removed_trie_member_hint` replacement hint.

## std::collections ordered type surface (ADR-046 / ADR-087, 2026-10-29)

The 37 `std::collections::ordered` free functions split by lifecycle in the
[#917](https://github.com/wogikaze/arukellt/issues/917) ordered-group wave:
27 `experimental` entries were deleted under ADR-014 D4, and the 10 `stable`
entries are deprecated through the ADR-087 window. The replacement for both
groups is the receiver surface on `SortedMap`, `BTreeMap`, `BTreeSet`,
`IndexMap`, `IndexSet`, and `BitSet`.

### Retired (27 experimental frees)

These entries were `experimental`, so no deprecation window or W0009 entry
applies. Qualified old calls fail with **E0100** and the
`resolver::removed_names::removed_ordered_member_hint` replacement hint.

| Removed free | Replacement |
|--------------|-------------|
| `ordered::btree_new()` | `BTreeMap::new()` |
| `ordered::btree_insert(m, k, v)` | `m.insert(k, v)` |
| `ordered::btree_get(m, k)` | `m.get(k)` |
| `ordered::btree_remove(m, k)` | `m.remove(k)` |
| `ordered::btree_contains_key(m, k)` | `m.contains_key(k)` |
| `ordered::btree_len(m)` | `m.len()` |
| `ordered::btree_keys(m)` | `m.keys()` (`Vec<i32>`, ascending) |
| `ordered::btree_range(m, s, e)` | `m.range(s, e)` (`[s, e)`) |
| `ordered::btree_set_new()` | `BTreeSet::new()` |
| `ordered::btree_set_insert(s, v)` | `s.insert(v)` (`bool`) |
| `ordered::btree_set_contains(s, v)` | `s.contains(v)` |
| `ordered::btree_set_len(s)` | `s.len()` |
| `ordered::btree_set_find_idx(s, v)` | `s.contains(v)` / `s.insert(v)` (no public index probe) |
| `ordered::index_map_new()` | `IndexMap::new()` |
| `ordered::index_map_insert(m, k, v)` | `m.insert(k, v)` |
| `ordered::index_map_get(m, k)` | `m.get(k)` |
| `ordered::index_map_len(m)` | `m.len()` |
| `ordered::index_map_keys(m)` | `m.keys()` (insertion order) |
| `ordered::index_set_new()` | `IndexSet::new()` |
| `ordered::index_set_insert(s, v)` | `s.insert(v)` (`bool`) |
| `ordered::index_set_contains(s, v)` | `s.contains(v)` |
| `ordered::index_set_len(s)` | `s.len()` |
| `ordered::bitset_set(bs, b)` | `bits.set(b)` |
| `ordered::bitset_clear(bs, b)` | `bits.clear(b)` |
| `ordered::bitset_with_capacity(n)` | `BitSet::with_capacity(n)` |
| `ordered::bitset_count(bs)` | `bits.count()` |
| `ordered::bitset_union(a, b)` | `a.union(b)` |
| `ordered::bitset_intersection(a, b)` | `a.intersection(b)` |

```ark
// Before
use std::collections::ordered
let mut m = ordered::btree_new()
ordered::btree_insert(m, 1, 100)

// After
use std::collections::ordered::BTreeMap
let mut m = BTreeMap::new()
m.insert(1, 100)
```

### Removed (10 stable frees, ADR-087 Wave 2)

The `sorted_map_*` and `bitset_new/mark/unmark/test` entries completed their
ADR-087 window and were deleted on 2026-10-29. A qualified call now fails with
**E0100** and the method hint below; W0009 no longer fires because the names do
not resolve. The 0.x exception applies under
[ADR-087](../adr/ADR-087-0x-deprecation-exception.md) D2: the Wave 1 W0009
coverage, this guide, and the changelog entry are the three required records.

| Removed free | Replacement |
|--------------|-------------|
| `ordered::sorted_map_new()` | `SortedMap::new()` |
| `ordered::sorted_map_len(m)` | `m.len()` |
| `ordered::sorted_map_find_idx(m, k)` | `m.find_idx(k)` |
| `ordered::sorted_map_get(m, k)` | `m.get(k)` |
| `ordered::sorted_map_contains(m, k)` | `m.contains(k)` |
| `ordered::sorted_map_insert(m, k, v)` | `m.insert(k, v)` |
| `ordered::bitset_new()` | `BitSet::new()` |
| `ordered::bitset_mark(bs, b)` | `bits.mark(b)` |
| `ordered::bitset_unmark(bs, b)` | `bits.unmark(b)` |
| `ordered::bitset_test(bs, b)` | `bits.test(b)` |

```ark
use std::collections::ordered::SortedMap
let mut m = SortedMap::new()
m.insert(1, 100)
```

### PriorityQueue<T> (7 pq frees)

The 7 `pq_*` free functions in `std::collections::linear` followed the same
ADR-087 Wave 1 deprecation / Wave 2 deletion in the 2026-10-29 collections
wave. The replacement is `PriorityQueue<T>` from
`use std::collections::pq::PriorityQueue`. The old frees used a
`[count, ...heap]` `Vec<i32>` layout and returned `i32` from `pop` / `peek`
(panicking when empty); the type owns `Vec<T>` and returns `Option<T>`.

| Removed free | Replacement |
|--------------|-------------|
| `linear::pq_new()` | `PriorityQueue::new<i32>()` |
| `linear::pq_push(q, x)` | `queue.push(x)` |
| `linear::pq_pop(q)` | `queue.pop()` (`Option<i32>`) |
| `linear::pq_peek(q)` | `queue.peek()` (`Option<i32>`) |
| `linear::pq_len(q)` | `queue.len()` |
| `linear::pq_is_empty(q)` | `queue.is_empty()` |
| `linear::pq_clear(q)` | `queue.clear()` |

```ark
use std::collections::pq::PriorityQueue
let mut q: PriorityQueue<i32> = PriorityQueue::new<i32>()
q.push(3)
match q.pop() {
    Option::Some(value) => {},
    Option::None => {},
}
```

### Retired source-only `vec::new_i32`

`std::collections::vec::new_i32()` was an unregistered public source helper
next to the `impl Vec<T>` block. It was retired with an **E0100** hint; the
spelling is `Vec::new<i32>()`.

### Behaviour correction in the shared store

The sorted pair vector used by `SortedMap` and `BTreeMap` shifted entries with
a key-only offset: inserting or removing a key in the middle could move a key
without its value and leave a stale pair behind. The ordered surface moves
whole `(key, value)` pairs now, and
`tests/fixtures/stdlib_collections_ordered/{btree_basic,btree_keys,ordered_map_methods}`
and `tests/fixtures/stdlib_ordered/ordered_basic` pin the corrected values as
well as the key order. The deprecated free
functions share the corrected helpers, so they receive the same fix during
their window.

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
`binary_search_by_key`. The no-argument algorithms are `Vec<T: Ord>` methods:
the free `std::collections::sort` module (`sort`, `sort_unstable`,
`is_sorted`, `select_nth`) was deleted and bare calls fail with E0100.

The `i32`, `i64`, and `String` lanes are verified on both targets by
`tests/fixtures/stdlib_vec/vec_sort_ord_i64.ark` /
`vec_sort_ord_string.ark` and their `_wasm32` counterparts, including 5+
elements that exercise scratch growth. Custom-struct elements with a
parametric `Ord` impl are covered by
`tests/fixtures/stdlib_trait/ord_sort_custom_struct.ark` and its `_wasm32`
counterpart. `f64` is not `Ord`, so use `v.sort_by(total_cmp)` as before.
The earlier `i64` / `String` scratch-layout failure on the linear and gc
core/test paths and the `Ord::cmp` trap for custom structs (#965) are
resolved.

`String` ordering operators (`<`, `>`, `<=`, `>=`) compare UTF-8 bytes and
agree with `a.cmp(b)` on both targets. They no longer depend on
`use std::core::cmp`; that import is still required for explicit `cmp` calls
and `Ordering` values. `tests/fixtures/operators/string_order.ark` and its
`_wasm32` counterpart cover all four operators.

`String` equality operators (`==`, `!=`) use the same UTF-8 byte comparison,
so values built through different paths (`"ap".concat("ple")`,
`String_from("apple")`, literals) compare by content on both targets. The
linear target previously compared String refs/pointers. `eq(a, b)` keeps the
same contract; `tests/fixtures/operators/string_eq.ark` and its `_wasm32`
counterpart cover both forms.

| Removed free call | Replacement |
|-------------------|-------------|
| `sort::sort(v)` | `v.sort()` |
| `sort::sort_unstable(v)` | `v.sort_unstable()` |
| `sort::is_sorted(v)` | `v.is_sorted()` |
| `sort::select_nth(v, k)` | `v.select_nth(k)` |

`dedup` remains the #697 residual; use `v.dedup_by(cmp)` meanwhile.

Window and chunk helpers (`windows`, `chunks`, `chunks_exact`) return an eager
`Vec<Vec<T>>`; `resize` and `extend` copy element handles without a `Clone`
bound, and `drain` / `splice` return the removed elements as an owned `Vec<T>`.

### `panic`

`panic(msg)` is no longer a prelude free function. The stable control
operation is the associated function on the `Std` namespace type
([ADR-090](../adr/ADR-090-std-namespace-and-panic-association.md)):

```ark
Std::panic(String_from("unreachable"))
```

`Std` is auto-imported with the v2 stable prelude; no `use` statement is
needed. A bare `panic(msg)` fails with E0100 and a hint naming `Std::panic`.

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

### HashSet<i32> / HashSet<String> Methods

`HashSet<i32>` is the canonical surface for membership and aggregate
operations over the flat hash set store, and `HashSet<String>` covers the
membership lane over the `Vec<String>` store (#917). The 20
`hash_set::hashset_i32_*` / `hash_set::hashset_str_*` adapters were retired on
2026-10-27; qualified calls now fail resolution with an E0100 hint that points
at the method surface.

| Old API | Replacement |
|---------|-------------|
| `hash_set::hashset_i32_new()` | `HashSet::new<i32>()` |
| `hash_set::hashset_i32_insert(s, v)` | `s.insert(v)` |
| `hash_set::hashset_i32_contains(s, v)` | `s.contains(v)` |
| `hash_set::hashset_i32_remove(s, v)` | `s.remove(v)` |
| `hash_set::hashset_i32_len(s)` | `s.len()` |
| `hash_set::hashset_i32_is_empty(s)` | `s.is_empty()` |
| `hash_set::hashset_i32_to_vec(s)` | `s.to_vec()` |
| `hash_set::hashset_i32_union(a, b)` | `a.union(b)` |
| `hash_set::hashset_i32_intersection(a, b)` | `a.intersection(b)` |
| `hash_set::hashset_i32_difference(a, b)` | `a.difference(b)` |
| `hash_set::hashset_i32_capacity(s)` | `s.capacity()` |
| `hash_set::hashset_i32_reserve(s, n)` | `s.reserve(n)` |
| `hash_set::hashset_i32_shrink_to_fit(s)` | `s.shrink_to_fit()` |
| `hash_set::hashset_i32_clear(s)` | `s.clear()` |
| `hash_set::hashset_i32_extend(s, other)` | `s.extend(other)` |
| `hash_set::hashset_str_new()` | `HashSet::new<String>()` |
| `hash_set::hashset_str_insert(s, v)` | `s.insert(v)` |
| `hash_set::hashset_str_contains(s, v)` | `s.contains(v)` |
| `hash_set::hashset_str_remove(s, v)` | `s.remove(v)` |
| `hash_set::hashset_str_len(s)` / `is_empty` | `s.len()` / `s.is_empty()` |

Import the type explicitly: `use std::collections::hash_set::HashSet`.
Both element families implement `FromIterator<VecIter<T>>`, `IntoIterator`,
`Clone`, `Eq`, and `Debug`, and `HashSet<String>` exposes the `to_vec`
snapshot the traits share. `HashSet::from_iter(values.iter())` is the source
spelling: the resolver exposes the trait function on the base type, and the
concrete `HashSet_i32::from_iter__VecIter_i32` /
`HashSet_String::from_iter__VecIter_i32` impl is selected from the cursor's
element type. `HashSet<i32>::from_iter` and `HashSet<i32>::clone` remain
invalid source syntax; write `HashSet::from_iter` / `HashSet::new<i32>()` and
let the method resolver pick the store. Iteration snapshots through `to_vec`
for both families, so `for value in set` yields the set members, not the flat
bucket layout.

`HashSet<String>` also covers `union`, `intersection`, `difference`, `clear`,
and `extend` over the same `Vec<String>` store. The algebra returns a new set
and leaves both operands intact, and `extend` inserts from a snapshot, so
`s.extend(s)` terminates. `capacity` / `reserve` / `shrink_to_fit` remain
`i32`-only: the String store is a linear `Vec<String>` with no bucket layout,
so it has no capacity to report or reserve. Method lookup is family-aware, so
a call on a `HashSet<String>` is rejected at typecheck with `E0200`
(``method `capacity` is not available on `HashSet<String>`; it is defined only
for `HashSet<i32>```) instead of resolving to the i32 store helper and
trapping at runtime. There is no `hash_set::` replacement gap here — those
adapters were retired with the rest.

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

### `std::collections::hash` HashSet Frees

The 21 `hash::hashset_*` / `hash::hashset_str_*` free functions were the flat
`Vec<i32>` / `Vec<String>` store layer behind the `HashSet<T>` methods. They
were deprecated with W0009 under the [ADR-087](../adr/ADR-087-0x-deprecation-exception.md)
0.x exception and deleted in the same #917 wave (2026-10-29); the declarations
stay as non-public MIR lowering targets, and qualified old calls fail with
E0100 and a method hint. Import `use std::collections::hash_set::HashSet` for
the receiver spelling.

| Old API | Replacement |
|---------|-------------|
| `hash::hashset_new()` | `HashSet::new<i32>()` |
| `hash::hashset_insert(s, v)` | `s.insert(v)` |
| `hash::hashset_contains(s, v)` | `s.contains(v)` |
| `hash::hashset_remove(s, v)` | `s.remove(v)` |
| `hash::hashset_len(s)` | `s.len()` |
| `hash::hashset_is_empty(s)` | `s.is_empty()` |
| `hash::hashset_to_vec(s)` | `s.to_vec()` |
| `hash::hashset_union(a, b)` | `a.union(b)` |
| `hash::hashset_intersection(a, b)` | `a.intersection(b)` |
| `hash::hashset_difference(a, b)` | `a.difference(b)` |
| `hash::hashset_capacity(s)` | `s.capacity()` |
| `hash::hashset_reserve(s, n)` | `s.reserve(n)` |
| `hash::hashset_shrink_to_fit(s)` | `s.shrink_to_fit()` |
| `hash::hashset_clear(s)` | `s.clear()` |
| `hash::hashset_extend(s, other)` | `s.extend(other)` |
| `hash::hashset_str_new()` | `HashSet::new<String>()` |
| `hash::hashset_str_insert(s, v)` | `s.insert(v)` |
| `hash::hashset_str_contains(s, v)` | `s.contains(v)` |
| `hash::hashset_str_remove(s, v)` | `s.remove(v)` |
| `hash::hashset_str_len(s)` | `s.len()` |
| `hash::hashset_str_is_empty(s)` | `s.is_empty()` |

### `std::collections::hash` HashMap Store Frees (ADR-087 Window)

The 21 `hash::hashmap_*` free functions were the flat `Vec<i32>` store layer
behind `HashMap<i32, i32>`. They were deprecated with W0009 under the
[ADR-087](../adr/ADR-087-0x-deprecation-exception.md) 0.x exception and
deleted in the same #917 wave (2026-10-29); the declarations stay as
non-public MIR lowering targets, and qualified old calls fail with E0100 and
a method hint. The replacement is `HashMap::new<i32, i32>()` /
`HashMap::with_capacity<i32, i32>(n)` plus the receiver methods. The capacity
/ bulk-maintenance group
(`with_capacity`, `capacity`, `reserve`, `try_reserve`, `shrink_to_fit`,
`clear`, `drain`, `remove_entry`, `extend`) is implemented for
`HashMap<i32, i32>` only: the String and fn-valued stores reject those calls
at typecheck with `E0200` instead of lowering over the wrong bucket layout.

| Old API | Replacement |
|---------|-------------|
| `hash::hashmap_new()` | `HashMap::new<i32, i32>()` |
| `hash::hashmap_get(m, k)` | `m.get(k)` (`Option<i32>`; the old sentinel was `0`) |
| `hash::hashmap_get_option(m, k)` | `m.get(k)` |
| `hash::hashmap_contains(m, k)` | `m.contains_key(k)` |
| `hash::hashmap_set(m, k, v)` | `m.insert(k, v)` (returns the previous `Option<i32>`, not `bool`) |
| `hash::hashmap_insert(m, k, v)` | `m.insert(k, v)` |
| `hash::hashmap_size(m)` | `m.len()` |
| `hash::hashmap_capacity(m)` | `m.capacity()` |
| `hash::hashmap_with_capacity(n)` | `HashMap::with_capacity<i32, i32>(n)` |
| `hash::hashmap_reserve(m, n)` | `m.reserve(n)` |
| `hash::hashmap_try_reserve(m, n)` | `m.try_reserve(n)` (`true` means the reserve completed) |
| `hash::hashmap_shrink_to_fit(m)` | `m.shrink_to_fit()` |
| `hash::hashmap_is_empty(m)` | `m.is_empty()` |
| `hash::hashmap_clear(m)` | `m.clear()` |
| `hash::hashmap_keys(m)` | `m.keys()` |
| `hash::hashmap_values(m)` | `m.values()` |
| `hash::hashmap_entries(m)` | `m.entries()` (`Vec<(i32, i32)>`, not the flat `Vec<i32>`) |
| `hash::hashmap_drain(m)` | `m.drain()` (`Vec<(i32, i32)>`, not the flat `Vec<i32>`) |
| `hash::hashmap_remove(m, k)` | `m.remove(k)` |
| `hash::hashmap_remove_entry(m, k)` | `m.remove_entry(k)` (`Option<(i32, i32)>`, not a flat `Vec<i32>`) |
| `hash::hashmap_extend(m, other)` | `m.extend(other)` |

**Before:**

<!-- skip-doc-check reason="historical removed-API example" owner="#917" kind="non-runnable" expires="2026-10-31" -->
```ark
let m = hash::hashmap_with_capacity(4)
let _ = hash::hashmap_set(m, 1, 10)
let n = hash::hashmap_size(m)
let cap = hash::hashmap_capacity(m)
```

**After:**

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#917" kind="non-runnable" expires="2026-10-31" -->
```ark
let mut m: HashMap<i32, i32> = HashMap::with_capacity<i32, i32>(4)
let _ = m.insert(1, 10)
let n = m.len()
let cap = m.capacity()
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
