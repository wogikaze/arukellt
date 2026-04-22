# Stdlib Baseline: Collections Hash Hardening

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 607
**Parent**: #590
**Depends on**: 604
**Track**: stdlib
**Orchestration class**: blocked-by-upstream

---

## Summary

Child issue for #590 Phase 4 — Collections Hardening.

The current `std::collections::hash` primary facade conflates "missing key" with
"stored zero" and can silently fail on insert. This issue hardens the monomorphic hash
family without attempting full generic collections (which require #044 and #312).

---

## Scope

**In scope:**

**Hash policy:**
- Define one canonical integer/string/combine hash policy across `std::core::hash`
  and `std::collections::hash` — remove "same concept, different mix function" drift
- Document stability vs quality expectations explicitly

**Primary API correctness:**
- Primary `get`-style facade must not collapse "missing" and "stored zero"
  (use `Option` or explicit sentinel that cannot be confused with a stored value)
- Primary insert path must not silently fail when full
  — return explicit failure or resize; do not pretend success
- If resize/rehash is not ready, return an explicit failure surface

**Raw helper separation:**
- Flat `Vec<i32>` layout knowledge must not be the default user path
- Raw helpers remain available but must not be presented as the recommended surface

**Out of scope:**
- Full generic `HashMap<K, V>` with trait-based hashing (requires #044, #312, #512)
- Robin Hood hashing or advanced collision strategies — future work
- Concurrent / lock-free collections

---

## Primary paths

- `std/collections/hash.ark`
- `std/core/hash.ark`
- `tests/fixtures/` (hash fixtures)

## Allowed adjacent paths

- `std/manifest.toml`
- `docs/stdlib/modules/collections.md`
- `benchmarks/` (if hash occupancy/collision benchmarks already exist)

---

## Upstream / Depends on

604 (contract honesty — facade/raw boundary must be named before correctness fixes)

## Blocks

- #608 (docs/bench closeout)
- #044 is NOT a dependency — this issue works on the monomorphic hash family only

---

## Acceptance

1. Primary `get` facade returns `Option<V>` or equivalent, not conflating missing with stored zero
2. Primary insert path never silently discards a write
3. Canonical hash policy is documented: one mix function, one integer policy
4. Raw layout helpers are named/placed to signal they are not the recommended path
5. Existing hash fixtures still pass; at least one new correctness fixture added

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
mise bench
```

---

## STOP_IF

- Do not implement generic `HashMap<K, V>` in this issue — that requires compiler support
- Do not add Robin Hood or other advanced strategies
- Do not resize/rehash if it would destabilize the monomorphic surface

---

## Close gate

Close when: primary facade cannot produce silent data loss, canonical hash policy is
documented, raw helpers are separated, and fixtures demonstrate the corrected behavior.
