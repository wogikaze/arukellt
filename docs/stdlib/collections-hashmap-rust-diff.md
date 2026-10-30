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
sees the bucket layout.

`HashSet<String>` shares that String-keyed bucket store: it is a key view
with the value slot fixed at 0, so `capacity`, `reserve`, `try_reserve`,
`shrink_to_fit`, `clear`, and `extend` lower to the same counters and
load-factor rehash as `HashMap<String, i32>`. No parallel linear facade is
kept.

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
`retain`, and `remove_entry`. Arukellt v2 has no general `&T` / `&mut T`, so
the source surface uses owned snapshots and an owned entry handle instead
(ADR-093):

- `m.iter() -> HashMapIter<K, V>` yields snapshot `(K, V)` pairs. The cursor
  owns the key/value snapshots, so later map writes are not observed. Pair
  order is implementation-defined bucket order.
- `for pair in m` consumes the same `HashMapIter<K, V>` snapshot through
  `IntoIterator`, so its item is an owned `(K, V)` pair and its type matches
  `m.iter()`. This is an explicit break from the earlier keys-only
  `for k in m` contract; update such loops to `for (k, v) in m` or
  `for pair in m.iter()`.
- `m.entry(key) -> Entry<K, V>` returns an owned handle that holds the map and
  key. `key()`, `or_insert(value)`, `or_insert_with(make)`, and `or_default()`
  resolve the value when the handle is consumed, and write through the map
  handle when the key is absent. `or_default()` writes `0` for `i32` values and
  the empty `String` for `String` values; fn-valued pairs have no default and
  expose only `or_insert` / `or_insert_with`. Keep the handle inside the map's
  lifetime.
- `m.retain(keep: fn((K, V)) -> bool)` evaluates the predicate against
  key/value snapshots and then removes rejected keys. The single owned-pair
  argument is the sound form without reference pairs; it is not Rust's
  `retain(|&k, &v| ...)`.
- `m.keys() -> Vec<K>`, `m.values() -> Vec<V>`, `m.entries() -> Vec<(K, V)>`,
  `m.drain() -> Vec<(K, V)>`, and `m.remove_entry(key) -> Option<(K, V)>` stay
  as eager snapshots.
- `HashMap::from_iter(m.iter())` and `HashMap::from_iter(m.into_iter())`
  collect the same snapshot cursor into a map. Each supported pair owns its
  concrete `FromIterator<HashMapIter<...>>` impl, so the call spelling carries
  the pair through the cursor type.

`HashMapIter<K, V>` owns separate `Vec<K>` / `Vec<V>` snapshots so its cursor
layout does not depend on tuple element lowering. Tuple-element `VecIter` is
supported on both targets, but adopting it as the map cursor would change the
`iter()` surface and is not part of this contract.

The deprecated flat frees used `Vec<i32>` snapshots
(`[key0, value0, key1, value1, ...]`); the typed method results are the
replacement, not a shape-compatible rename.

`retain` performs a snapshot pass plus per-key removal, which is O(n²) for the
flat `i32 -> i32` store in the worst case. A one-pass bucket rebuild is a
separate store optimization.

## HashSet

Rust implements `HashSet<T>` as a set facade over `HashMap<T, ()>`.
Arukellt uses:

- `HashSet<i32>` as the raw `i32 -> i32` map with sentinel value `1`.
- `HashSet<String>` as a key view over `HashMap<String, i32>` with sentinel
  value `0`.

The String set therefore shares the String-keyed map's bucket arrays and
0.75 load-factor growth instead of keeping a parallel `Vec<String>` facade.
Its `to_vec`, `iter`, `IntoIterator`, `Clone`, and `Debug` snapshot in bucket
order, which is implementation-defined; compare sets with `Eq`, not iteration
order.

Both element families expose `iter() -> VecIter<T>` over a `to_vec` snapshot
and `retain(keep: fn(T) -> bool)`, which evaluates the predicate against the
snapshot before reinserting kept elements. A true unit-value set
representation should be addressed with a later collection representation
cleanup.
