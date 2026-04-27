---
Status: done
Created: 2026-04-18
Updated: 2026-04-22
ID: 528
Track: stdlib
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v{N}: none
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Source: follow-up extraction from `#519`
---

# Stdlib collections/hash: property と differential tests を map / set invariant で拡張する

## Summary

`std::collections::hash` family は regression fixtures がある一方で、map / set の property invariant と differential replay を focused queue item として追跡していない。
この issue は disjoint-key commutativity, insert/remove balance, deterministic snapshot comparison を分離して、hash family の振る舞い差分を継続検証する。

## Why this must exist

- `#519` の matrix で collections/hash は property-differential の最重要対象として挙がっている
- 既存の hashmap / hashset fixtures だけでは、オペレーション列 replay と invariant 検証の継続監査が足りない
- hash family は iteration order をそのまま比べると不安定なので、comparison rules を issue として固定しておく必要がある

## Evidence source

- `issues/open/519-stdlib-parity-property-and-differential-tests.md`
- `docs/stdlib/modernization/514-parser-host-quality-audit.md`
- `std/core/hash.ark`
- `std/collections/hash.ark`

## Primary paths

- `std/collections/hash.ark`
- `std/collections/hash_map.ark`
- `std/collections/hash_set.ark`
- `std/core/hash.ark`
- `tests/fixtures/stdlib_hashmap/`
- `tests/fixtures/stdlib_core/hash.ark`
- `crates/arukellt/tests/`

## Non-goals

- hash algorithm redesign
- performance tuning or benchmark work
- API naming / facade cleanup
- cross-family JSON / TOML / CSV coverage

## Acceptance

- [x] property invariants are defined for at least disjoint-key insert commutativity and insert/remove balance
- [x] differential replay is defined against a reference snapshot that does not depend on iteration order
- [x] a deterministic seed / snapshot normalization rule is documented for the test harness
- [x] the corpus or Rust test module is wired into the repo without changing production hash behavior

## Evidence

- `tests/fixtures/stdlib_hashmap/property_disjoint_commutativity.ark` — disjoint-key insert commutativity (A vs B under reversed insertion order with fixed-probe snapshot).
- `tests/fixtures/stdlib_hashmap/property_insert_remove_balance.ark` — insert N=16 then remove N=16 leaves `size == 0`, `is_empty == true`, every key absent.
- `tests/fixtures/stdlib_hashmap/property_snapshot_normalized.ark` — differential replay: two maps built with different insertion orders produce byte-identical stdout under the documented normalization rule (collect `hashmap_keys` → inline ascending insertion sort → emit `key`/`value` lines in sorted order).
- Registered in `tests/fixtures/manifest.txt` under the `run:` section for `stdlib_hashmap`.
- Zero diff under `std/collections/hash*.ark` and `std/core/hash.ark` in the slice commit (acceptance item 4).

## Required verification

- focused collections/hash property or differential test run
- `python scripts/manager.py verify fixtures`
- `python3 scripts/gen/generate-issue-index.py`

## Close gate

- invariant coverage is repo-provable from the new tests
- differential comparison avoids iteration-order flakiness
- the issue can be closed by citing corpus / harness evidence from repo state

## Close note

Closed by commit `e791a84a` on branch `feat/528-stdlib-hash-property-tests`.
All four acceptance criteria met: three property/differential fixtures added and
passing, registered in manifest, no production hash source changed.
