---
Status: open
Created: 2026-06-25
Updated: 2026-06-30
ID: 687
Track: stdlib-api
Depends on: 495
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: HashMap / HashSet Rust parity gap audit 2026-06-25
---

# 687 — HashMap / HashSet Rust parity gap audit and closure

## Summary

Current `HashMap` / `HashSet` support is useful but substantially narrower than
Rust's `std::collections` contract. Rust exposes generic
`HashMap<K, V, S = RandomState, A = Global>` and
`HashSet<T, S = RandomState, A = Global>` types, backed by SwissTable-style
probing and a pluggable/randomized hasher. Arukellt currently has:

- flat `Vec<i32>` storage for `std::collections::hash` i32 maps/sets:
  `[capacity, size, keys..., values..., flags...]`
- open addressing with linear probing, fixed capacity unless recreated
- wrapper APIs for `HashMap<i32,i32>`, `HashMap<String,i32>`,
  `HashMap<i32,String>`, and `HashMap<String,String>`
- `HashSet<i32>` implemented through `HashMap<i32,i32>` with sentinel value `1`
- `HashSet<String>` facade backed by `Vec<String>` where insert/contains/remove
  are currently stubs returning `false`

This issue tracks the concrete Rust parity gaps, then either closes them or
documents intentional Arukellt differences.

## Rust behavior baseline

Rust standard library baseline as of Rust `std` 1.96.0 docs:

- `HashMap` is generic over key, value, hasher, and allocator.
- Keys require coherent `Eq` and `Hash`; borrowed-key lookup is supported through
  `Borrow<Q>` for `get`, `contains_key`, and `remove`.
- Default hashing is randomly seeded and intended to resist HashDoS. The current
  default algorithm is documented as SipHash 1-3, subject to change.
- The implementation is a Rust port of Google's SwissTable with quadratic
  probing and SIMD lookup.
- `HashMap::new()` starts at zero capacity and allocates lazily on insertion.
- Capacity management includes `capacity`, `reserve`, `try_reserve`,
  `shrink_to_fit`, and `shrink_to`.
- `insert` returns `Option<V>` with the replaced value.
- APIs include iteration (`iter`, `iter_mut`, `keys`, `values`, consuming
  variants), `entry`, `get_mut`, `get_key_value`, `remove_entry`, `retain`,
  `drain`, and `extract_if`.
- `HashSet` is implemented as a `HashMap` where the value is `()`, and exposes
  set operations and iterators over generic `T`.

## Current Arukellt gaps

| Area | Rust | Arukellt current state | Required decision |
|------|------|------------------------|-------------------|
| Genericity | Any `K: Eq + Hash`, any `V` | Enumerated concrete variants plus i32 raw layer | Implement generic surface or document supported concrete set |
| Hasher | `RandomState` default, custom `BuildHasher` | Deterministic FNV-like helpers / ad hoc i32 hash | Decide HashDoS posture and custom hasher scope |
| Storage | SwissTable, quadratic probing, SIMD lookup | Linear probing; i32 map is flat `Vec<i32>`; string maps use target-aware builtins | Keep simple implementation or add resize/tombstone/metadata parity |
| Capacity | Lazy allocation, reserve/shrink APIs | `hashmap_new` allocates capacity 16; `with_capacity` fixed-capacity; no resize | Add growth and capacity APIs or explicitly mark fixed-capacity |
| Insert return | `Option<V>` old value | i32 raw `hashmap_set` returns `bool`; wrappers return `()` | Normalize facade around `Option<V>` |
| Lookup | Borrowed lookup and references | By-value lookup, `Option` for some paths | Define by-value vs borrowed lookup contract |
| Remove | Tombstone/backshift-compatible table behavior | i32 remove rebuilds from keys/values; string map uses flag `2` tombstone | Align deletion invariants and tests |
| Iteration | Iterator views and consuming iterators | Snapshot `Vec<i32>` keys/values only for raw i32 maps | Add entries/iter docs or expose snapshot-only semantics |
| Entry API | `entry`, `or_insert`, `and_modify` | absent | Defer explicitly or implement minimal `get_or_insert` equivalent |
| Set value | `HashSet<T>` as `HashMap<T, ()>` | `HashSet<i32>` stores sentinel `1`; `HashSet<String>` facade stubs | Replace sentinel/stubs with real unit-value or dedicated set invariant |
| Trait impls | Clone, Default, Extend, FromIterator, IntoIterator, Eq, Debug, Index | absent or ad hoc builtins | Choose minimal trait surface for stdlib vNext |

## Acceptance

- [x] Add a user-facing `docs/stdlib/collections-hashmap-rust-diff.md` or
      equivalent section that describes intentional differences from Rust.
      - 2026-06-26: Added `docs/stdlib/collections-hashmap-rust-diff.md`
        documenting raw representation, deterministic hashing, capacity/growth
        behavior, snapshot entries/drain APIs, and deferred iterator/entry
        parity.
- [x] `HashSet<String>` insert/contains/remove are implemented or removed from
      the public facade until implemented.
      - 2026-06-25: `std::collections::hash_map::hashset_str_*` no longer
        returns fixed stub values; `hashset_string_facades.ark` covers both
        `hash::hashset_str_*` and the compatibility `hash_map::hashset_str_*`
        facade.
- [x] `HashMap` facade APIs consistently return `Option<V>` for replacement and
      lookup where the type system can represent it; legacy bool/unit behavior
      is documented as raw compatibility.
      - 2026-06-25: `std::collections::hash::hashmap_insert` now returns
        `Option<i32>`, and `std::collections::hash_map::*_insert` facade
        wrappers return the previous value for `HashMap<i32,i32>`,
        `HashMap<String,i32>`, `HashMap<i32,String>`, and
        `HashMap<String,String>`. The lower-level `hashmap_set` bool API remains
        the fixed-capacity raw store primitive.
- [x] Add resize/growth behavior for new insertions, or document fixed-capacity
      failure semantics and expose tests for full-table insertion failure.
      - 2026-06-25: `std::collections::hash` now grows the raw i32 HashMap
        before inserts exceed the 0.75 load-factor threshold, and exposes
        `hashmap_capacity`, `hashmap_reserve`, `hashmap_try_reserve`, and
        `hashmap_shrink_to_fit`. `hashmap_capacity_reserve.ark` covers reserve,
        automatic insertion growth, shrink, and value preservation.
- [x] Add fixtures covering collision, update-return, deletion-after-collision,
      zero-value lookup, full-table behavior, and string set membership.
      - 2026-06-26: Added `hashmap_parity_edges.ark` covering colliding inserts,
        `hashmap_insert` replacement returns, zero-value lookup,
        deletion-after-collision lookup, insertion growth beyond the old
        full-table case, flat entries/drain/remove-entry snapshots, and String
        HashSet membership through the compatibility facade.
- [x] Decide and document whether Arukellt will support Rust-like randomized
      hashing/custom hashers or keep deterministic hashing for reproducibility.
      - 2026-06-26: Documented deterministic FNV-1 hashing as the current
        contract; randomized seeding/custom hashers are deferred to a separate
        design task.
- [x] Decide and document whether `entry` / iterator / `FromIterator` parity is
      in scope for this issue or tracked separately.
      - 2026-06-26: Documented snapshot-style `hashmap_entries`,
        `hashmap_drain`, and `hashmap_remove_entry` as the current raw
        compatibility surface; Rust-style borrowed iterators, `entry`, `retain`,
        and `FromIterator` are deferred until the required abstraction layer
        exists.
- [x] Generated stdlib reference and name index are regenerated after any API
      surface change.
      - 2026-06-26: Regenerated manifest-backed stdlib docs after adding
        `hashmap_entries`, `hashmap_drain`, and `hashmap_remove_entry`.
- [ ] `python3 scripts/manager.py verify quick` exits 0.
      *(2026-06-30: 162/168 checks pass. Remaining 6 failures are all
      pre-existing runtime wasm crashes (arukellt-s2-runtime.wasm
      function 4163/548), unrelated to HashMap/HashSet. The #687-specific
      checks (fixture manifest sync, docs consistency, false-done hygiene,
      compiler boundary limits) all pass.)*

## References

- `std/collections/hash.ark`
- `std/collections/hash_map.ark`
- `std/collections/hash_set.ark`
- `std/collections/hash_string.ark`
- `std/manifest.toml`
- `issues/done/044-std-collections-hash.md`
- Rust `HashMap` docs: <https://doc.rust-lang.org/std/collections/struct.HashMap.html>
- Rust `HashSet` docs: <https://doc.rust-lang.org/std/collections/struct.HashSet.html>
