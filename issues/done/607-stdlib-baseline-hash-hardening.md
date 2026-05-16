---
Status: done
Created: 2026-04-22
Updated: 2026-05-16
ID: 607
Track: stdlib
Orchestration class: implementation-ready
Depends on: 604
---

# Stdlib Baseline: Collections Hash Hardening

Closure note (2026-05-16):

All acceptance criteria satisfied (implementation already present per 2026-05-14
recheck, re-verified on 2026-05-16):

1. **Primary get facade returns Option<V>** - `hashmap_get_option` is documented as the primary lookup API (returns `Option<i32>`). Legacy `hashmap_get` remains for backward compat but is called out as unable to distinguish missing from stored zero.
2. **Primary insert path never silently discards writes** - `hashmap_set` returns `bool`: `true` when stored, `false` when the fixed-capacity table is full. `hashset_insert` propagates `hashmap_set` failure.
3. **Canonical hash policy documented** - `std::collections::hash::hash_i32` uses the same stable byte-mixing policy as `std::core::hash::hash_i32`. Module doc comments document stability vs quality expectations.
4. **Raw layout helpers separated from recommended surface** - Raw helpers (`hashmap_new`, `hashmap_get`, `hashmap_set` with raw Vec<i32> layout) remain available but the module doc steers users toward `hashmap_get_option`.
5. **Existing hash fixtures pass; new correctness fixture added** - `tests/fixtures/stdlib_hashmap/hashmap_hardening.ark` covers: stored zero via `Some(0)`, missing via `None`, explicit full-table insert failure, unchanged size after failed insertion, and no false containment.

Changes made in prior work (re-verified):

- `std/collections/hash.ark`: Unified `hash_i32` with `std::core::hash::hash_i32` policy. Fixed linear probing to `(idx + 1) % cap`. `hashmap_set` returns bool. `hashset_insert` propagates failure. `hashmap_get_option` documented as primary API.

Verification:

- `python3 scripts/manager.py verify quick`: No new failures.
- `python3 scripts/manager.py verify fixtures`: No failing fixtures; hash fixtures compile correctly.
- `target/release/arukellt check` and `compile` pass for `hashmap_hardening.ark`.
