# HashMap / HashSet Rust Compatibility Notes

This page records the intentional differences between Arukellt's current
`std::collections` hash collections and Rust's `std::collections::HashMap` /
`HashSet` APIs.

## Current Representation

Arukellt's canonical source API is `HashMap::new<K,V>()` plus receiver methods
(`insert`, `get`, `contains_key`, `remove`, `len`, `is_empty`, `keys`,
`values`, `entries`, and the capacity / bulk-maintenance group). The compiler
statically monomorphizes the supported concrete pairs. The raw monomorphic
`i32 -> i32` implementation in `std::collections::hash` is the compatibility
and linear-memory implementation layer. It is backed by a flat `Vec<i32>`:

```text
[capacity, size, keys..., values..., flags...]
```

The compiler rewrites the generic methods to non-public implementation
functions in `std::collections::hash_string` / `hash_fn` / `hash_str_fn` for
the concrete GC pairs. The typed `hash_map::hashmap_*` adapter frees were
retired by #917; the receiver methods are the only user spelling.

The 21 `hash::hashmap_*` flat-store frees were retired through the ADR-087
two-phase lifecycle (W0009 deprecation, then deletion). Their declarations
remain as non-public MIR lowering targets, so qualified old calls fail with
E0100; the user spelling is the `HashMap<i32, i32>` method surface.

## Hashing Policy

Rust's default `HashMap` uses randomized seeding and supports custom hashers.
Arukellt currently uses deterministic FNV-1 style hashing for reproducible
compiler, fixture, and selfhost behavior.

Custom hashers and Rust-like randomized seeding are not part of the current
stdlib contract. They should be tracked as a separate language/runtime design
task if the project needs them later.

## Capacity And Growth

Every supported pair grows before inserts would exceed a 0.75 load-factor
threshold, and every pair exposes:

- `HashMap::with_capacity<K, V>(n)`
- `m.capacity()`
- `m.reserve(n)`
- `m.try_reserve(n)`
- `m.shrink_to_fit()`
- `m.clear()`
- `m.drain()` (returns `Vec<(K, V)>` and empties the map)
- `m.remove_entry(k)` (returns `Option<(K, V)>`)
- `m.extend(other)`

`try_reserve` returns `bool` because the current `Vec` implementation does not
expose a recoverable allocation-error channel comparable to Rust's
`TryReserveError`; `true` means the reserve completed.

The surface is accepted for every supported pair and lowered per pair.
`HashMap<i32, i32>` uses the flat `Vec<i32>` counters; the String and
fn-valued pairs keep their bucket arrays behind the non-public `__hm_*`
bridge, and `hash_string.ark` / `hash_fn.ark` / `hash_str_fn.ark` hold one
`with_capacity` / `capacity` / `reserve` / `try_reserve` / `shrink_to_fit` /
`clear` / `drain` / `remove_entry` / `extend` target per pair. The source
declarations stay on the flat pair and the typechecker's HashMap method table
accepts the other pairs, because the pinned bootstrap must still typecheck
`std/collections/hash_map.ark`; MIR picks the implementation from the
receiver's pair. `drain` and `remove_entry` return typed pairs, so no caller
sees the bucket layout. This does not weaken the separate `HashSet<String>`
boundary: that facade has no bucket layout and still rejects capacity
methods.

No capability was lost in the free-function retirement: the 21 retired
`hashmap_*` frees were i32-keyed and i32-valued.


## Replacement And Lookup

The compatibility direction is Rust-like `Option<V>` where the current type
system can represent it:

- `m.insert(k, v)` returns the previous `Option<V>` for every supported
  monomorphized `HashMap` pair.
- `m.get(k)` and `m.remove(k)` return `Option<V>`.
- `m.remove_entry(k)` returns `Option<(K, V)>`.

The deprecated raw `hashmap_set` returned `bool`; its method replacement is
`insert`, whose return value is the previous value as `Option`.

## Entries, Drain, And Iteration

Rust exposes borrow-based iterators and APIs such as `entry`, `iter`, `drain`,
`retain`, and `remove_entry`.

Arukellt does not yet have a stable borrowed iterator abstraction for these
collections. The method surface therefore exposes snapshot values:

- `m.keys() -> Vec<K>`
- `m.values() -> Vec<V>`
- `m.entries() -> Vec<(K, V)>`
- `m.drain() -> Vec<(K, V)>`, then empties the map
- `m.remove_entry(key) -> Option<(K, V)>`

The deprecated flat frees used `Vec<i32>` snapshots
(`[key0, value0, key1, value1, ...]`); the typed method results are the
replacement, not a shape-compatible rename.

Rust-style `entry`, borrowed iteration, `retain`, and `FromIterator` parity are
deferred until the language/runtime has the necessary abstraction layer.

## HashSet

Rust implements `HashSet<T>` as a set facade over `HashMap<T, ()>`.
Arukellt currently uses:

- `HashSet<i32>` as the raw `i32 -> i32` map with sentinel value `1`.
- `HashSet<String>` as a compatibility `Vec<String>` facade.

A true unit-value set representation should be addressed with a later
collection representation cleanup.
