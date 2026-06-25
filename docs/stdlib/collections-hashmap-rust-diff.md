# HashMap / HashSet Rust Compatibility Notes

This page records the intentional differences between Arukellt's current
`std::collections` hash collections and Rust's `std::collections::HashMap` /
`HashSet` APIs.

## Current Representation

Arukellt's source stdlib exposes a raw monomorphic `i32 -> i32` implementation
in `std::collections::hash`. It is backed by a flat `Vec<i32>`:

```text
[capacity, size, keys..., values..., flags...]
```

The user-facing generic `HashMap<K, V>` constructors and methods are compiler
builtins that are monomorphized for the currently supported concrete pairs.
Compatibility wrapper functions live in `std::collections::hash_map`.

## Hashing Policy

Rust's default `HashMap` uses randomized seeding and supports custom hashers.
Arukellt currently uses deterministic FNV-1 style hashing for reproducible
compiler, fixture, and selfhost behavior.

Custom hashers and Rust-like randomized seeding are not part of the current
stdlib contract. They should be tracked as a separate language/runtime design
task if the project needs them later.

## Capacity And Growth

The raw `i32 -> i32` map grows before inserts would exceed a 0.75 load-factor
threshold. It exposes:

- `hashmap_capacity`
- `hashmap_reserve`
- `hashmap_try_reserve`
- `hashmap_shrink_to_fit`

`hashmap_try_reserve` returns `bool` because the current `Vec` implementation
does not expose a recoverable allocation-error channel comparable to Rust's
`TryReserveError`.

## Replacement And Lookup

The compatibility direction is Rust-like `Option<V>` where the current type
system can represent it:

- `hashmap_insert` returns the previous `Option<i32>`.
- `hashmap_get_option` and `hashmap_remove` return `Option<i32>`.
- `hash_map::*_insert` wrappers return the previous value for supported
  monomorphized `HashMap` pairs.

The older raw `hashmap_set` remains as a compatibility primitive returning
`bool` to report whether storage succeeded.

## Entries, Drain, And Iteration

Rust exposes borrow-based iterators and APIs such as `entry`, `iter`, `drain`,
`retain`, and `remove_entry`.

Arukellt does not yet have a stable borrowed iterator abstraction for these
collections. The raw map therefore exposes snapshot-style flat vectors:

- `hashmap_keys(m) -> Vec<i32>`
- `hashmap_values(m) -> Vec<i32>`
- `hashmap_entries(m) -> Vec<i32>` as `[key0, value0, key1, value1, ...]`
- `hashmap_drain(m) -> Vec<i32>` with the same flat shape, then clears `m`
- `hashmap_remove_entry(m, key) -> Vec<i32>` as `[key, value]` or empty

Rust-style `entry`, borrowed iteration, `retain`, and `FromIterator` parity are
deferred until the language/runtime has the necessary abstraction layer.

## HashSet

Rust implements `HashSet<T>` as a set facade over `HashMap<T, ()>`.
Arukellt currently uses:

- `HashSet<i32>` as the raw `i32 -> i32` map with sentinel value `1`.
- `HashSet<String>` as a compatibility `Vec<String>` facade.

A true unit-value set representation should be addressed with a later
collection representation cleanup.
