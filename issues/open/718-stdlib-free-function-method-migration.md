---
Status: open
Created: 2026-07-05
Updated: 2026-07-05
ID: 718
Track: stdlib-api
Depends on: "700, 701"
Orchestration class: incremental
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Stdlib free-function inventory 2026-07-05
---

# 718 — Stdlib free-function → method/trait migration inventory

## Summary

The stdlib has ~287 free functions that should be migrated to trait/method
syntax per ADR-036 and issue #709. Now that #700 (builtin method syntax)
and #701 (associated function syntax) are done, the migration can proceed
in tiers.

This issue provides the **complete inventory** (required by #709) and
tracks the per-type migration milestones.

## Current state

### Existing `impl` blocks (migration targets already available)

- `impl i32`: `to_string`, `abs`, `min`, `max`
- `impl i64`: `to_string`
- `impl f64`: `to_string`
- `impl bool`: `to_string`
- `impl char`: `to_string`
- `impl String`: `len`, `char_at`, `index_of`, `slice`, `concat`, `clone`,
  `starts_with`, `ends_with`, `contains`, `to_lower`, `to_upper`, `trim`,
  `replace`
- `impl Vec<T>`: `push`, `pop`, `get`, `set`, `len`, `is_empty`,
  `get_unchecked`
- `Ord` trait: `cmp` on i32/i64/f64/String/char/bool
- `Hash` trait: `hash` on i32/String/i64/bool/char/f64
- `Display`/`Debug` traits: on i32/i64/f64/bool/char/String

### Blocked by upstream

- `Iterator` trait (#691) — blocked by #688, #707
- `Ord`/`PartialOrd` trait extensions (#695) — blocked by #688
- `Vec<T>` operation extension (#697) — blocked by #691, #695

## Migration tiers

### Tier 1: Duplicated by existing trait/impl (~20 functions)

Free functions that have **both** a free function AND an existing
trait/inherent method. These can be deprecated immediately.

**`std/core/cmp.ark`**:
- `cmp(a, b)` → `a.cmp(b)` (Ord trait exists)
- `min(a, b)` → `a.min(b)` (inherent impl exists on i32)
- `max(a, b)` → `a.max(b)` (inherent impl exists on i32)
- `clamp(x, lo, hi)` → `x.clamp(lo, hi)` (needs inherent impl)

**`std/core/convert.ark`**:
- `i32_to_string(x)` → `x.to_string()` (Display/inherent exists)
- `i64_to_string(x)` → `x.to_string()`
- `f64_to_string(x)` → `x.to_string()`
- `bool_to_string(x)` → `x.to_string()`

**`std/core/math.ark`**:
- `abs(x)` → `x.abs()` (inherent exists on i32)
- `min(a, b)` → `a.min(b)` (inherent exists on i32)
- `max(a, b)` → `a.max(b)` (inherent exists on i32)
- `sqrt(x)` → `x.sqrt()` (needs inherent impl on f64)
- `clamp(x, lo, hi)` → `x.clamp(lo, hi)` (needs inherent impl on i32)
- `pow_i32(base, exp)` → `base.pow(exp)` (needs inherent impl on i32)
- `is_power_of_two(n)` → `n.is_power_of_two()` (needs inherent impl)
- `next_power_of_two(n)` → `n.next_power_of_two()` (needs inherent impl)
- `leading_zeros(n)` → `n.leading_zeros()` (needs inherent impl)
- `trailing_zeros(n)` → `n.trailing_zeros()` (needs inherent impl)
- `popcount(n)` → `n.popcount()` (needs inherent impl)
- Deprecated wrappers to remove: `abs_i32`, `min_i32`, `max_i32`

**`std/core/hash.ark`**:
- `hash_i32(value)` → `value.hash()` (Hash trait exists)
- `hash_string(value)` → `value.hash()`
- Deprecated wrapper to remove: `hash_combine`

### Tier 2: Single receiver, needs new impl (~27 functions)

**`std/core/either.ark`** (9 functions → `impl Either`):
- `is_left`, `is_right`, `from_left`, `from_right`, `left_to_option`,
  `right_to_option`, `swap`, `map_left`, `map_right`, `either_fold`

**`std/core/error.ark`** (1 function → `impl Error`):
- `error_message` → `e.message()`

**`std/io/mod.ark`** (~16 functions → Reader/Writer/BufReader/BufWriter):
- Reader: `reader_eof`, `reader_read`, `reader_read_exact`,
  `reader_read_all`, `reader_read_line`
- Writer: `writer_write`, `writer_write_str`, `write_all`,
  `write_string`, `writer_flush`, `writer_to_bytes`
- BufReader: `buf_reader_read_line`
- BufWriter: `buf_writer_write_str`, `buf_writer_flush`
- Memory buffer: `write_bytes`, `read_bytes`, `seek_to`, `position`,
  `buffer_len`, `fill_buffer`
- FileHandle: `file_read`, `file_write`

### Tier 3: Collections (~60 functions)

**`std/collections/hash_map.ark`** (31 functions → `impl HashMap<K,V>`):
- All `hashmap_*` and `hashset_str_*` monomorphizations

**`std/collections/sort.ark`** (10 functions → `impl Vec<T>`):
- `sort`, `sort_by`, `sort_by_key`, `sort_unstable`, `sort_unstable_by`,
  `sort_unstable_by_key`, `partition`, `is_sorted`, `is_sorted_by`,
  `select_nth`

**`std/collections/linked_list.ark`** (11 functions → new `LinkedList` type)

**`std/collections/trie.ark`** (7 functions → new `Trie` type)

**`std/collections/vec.ark`** (1 function):
- `new_i32()` → `Vec::new()` associated function

### Tier 4: Text/Bytes (~80 functions)

**`std/text/string.ark`** (21 functions → extend `impl String`):
- `split`, `join`, `starts_with`, `ends_with`, `contains`, `to_lower`,
  `to_upper`, `len_bytes`, `len_chars`, `is_empty`, `slice_bytes`,
  `lines`, `trim`, `trim_start`, `trim_end`, `replace`, `repeat`,
  `chars`, `from_utf8`, `to_utf8_bytes`

**`std/text/builder.ark`** (6 functions → new `StringBuilder` type)

**`std/text/fmt.ark`** (5 functions → extend inherent impls)

**`std/text/rope.ark`** (8 functions → new `Rope` type, experimental)

**`std/bytes/mod.ark`** (~40 functions → new `Bytes`/`ByteBuf`/`ByteCursor` types)

### Tier 5: Prelude wrappers (~100 functions)

Most are thin wrappers re-exporting the above. Once source modules
migrate, these wrappers should be deprecated. Compiler intrinsics
(`println`, `print`, `len`, `push`, `get`, `pop`, etc.) need compiler
support to become methods — track separately.

### Keep as free functions (~50 functions)

- Constructors / associated functions: `vec_iter`, `map_iter`,
  `stdin()`, `stdout()`, `writer_new()`, `buf_new()`, `Vec::new()`
- Path-based fs: `read_string`, `write_string`, `exists`, `is_file`,
  `is_dir`, `read_dir`, `metadata`
- Global host ops: `args()`, `var()`, `print()`, `println()`, `exit()`,
  `random_i32()`, `monotonic_now()`, `now_ms()`
- Binary ops with no clear receiver: `gcd`, `lcm`, `combine`, `copy_bytes`
- Parse functions: `parse_i32`, `parse_i64`, `parse_f64`
- HTTP/sockets: `request`, `get`, `serve`, `connect`, `listen`, `send`

## Required work

### Phase 1 (this issue, Tier 1 only)

- [ ] Add missing inherent methods to `impl i32`: `clamp`, `pow`,
      `is_power_of_two`, `next_power_of_two`, `leading_zeros`,
      `trailing_zeros`, `popcount`
- [ ] Add missing inherent methods to `impl f64`: `sqrt`, `clamp`
- [ ] Add missing inherent methods to `impl i64`: `abs`, `min`, `max`
- [ ] Deprecate duplicated free functions in `std/core/cmp.ark`,
      `std/core/convert.ark`, `std/core/math.ark`, `std/core/hash.ark`
- [ ] Remove deprecated wrappers: `abs_i32`, `min_i32`, `max_i32`,
      `hash_combine`
- [ ] Update in-tree callers to use method syntax
- [ ] Update `std/manifest.toml` deprecation metadata
- [ ] `python scripts/manager.py verify quick` passes

### Phase 2 (future, Tier 2-3, blocked by #691, #695)

- [ ] Migrate Either, Error, IO functions to methods
- [ ] Migrate HashMap, sort, LinkedList, Trie to methods/new types

### Phase 3 (future, Tier 4-5, blocked by Phase 2)

- [ ] Migrate text/bytes functions to methods
- [ ] Deprecate prelude wrappers after sources migrate

## Acceptance

- [ ] Tier 1 free functions are deprecated or removed
- [ ] Missing inherent methods added to `impl i32`, `impl f64`, `impl i64`
- [ ] In-tree callers use method syntax for Tier 1 operations
- [ ] `std/manifest.toml` reflects deprecation status
- [ ] `python scripts/manager.py verify quick` passes
- [ ] This issue provides the inventory data that #709 requires

## References

- #700 (builtin method syntax — done)
- #701 (associated function syntax — done)
- #703 (monomorphic API bold cutover — blocked by #691, #695)
- #709 (trait-first API policy — blocked by #691, #695, #697, #703)
- ADR-036 (trait-stdlib-redesign)
- `std/core/cmp.ark`, `std/core/convert.ark`, `std/core/math.ark`,
  `std/core/hash.ark`
- `std/collections/string.ark`, `std/collections/vec.ark`
