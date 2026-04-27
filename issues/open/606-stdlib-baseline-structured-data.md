---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 606
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Stdlib Baseline: Structured Data and Semantics Baseline
**Parent**: #590
**Depends on**: 604
**Track**: stdlib
**Orchestration class**: blocked-by-upstream

---

## Summary

Child issue for #590 Phase 3 — Structured Data / Semantics Baseline.

Fix contract ambiguity and semantic gaps in `std::json`, `std::toml`, `std::text`, and
`std::time` / `std::host::clock` boundary.

---

## Scope

**In scope:**

**JSON:**
- `std::json` parse semantics must be whole-document (not prefix/heuristic)
- Add negative-case fixtures for trailing garbage, malformed input
- Do not imply a DOM or streaming contract that is not implemented

**TOML:**
- Bounded subset must be explicitly documented
- Add fixtures for at-boundary behavior: key types, array of tables edge cases
- Mark partial/experimental explicitly

**Text:**
- `std::text` must clearly distinguish byte-based vs char/Unicode-aware operations
- `len_chars` must document its "best-effort" qualifier explicitly
- `to_lower`, `to_upper`, `trim_*` must document ASCII-only scope

**Time:**
- Split `std::time` (duration math) from `std::host::clock` (actual host time reads)
- Make the boundary explicit and documented
- Do not let `std::time` imply runtime host access it does not have

**Out of scope:**
- Full TOML 1.0 compliance
- Full Unicode normalization
- Timezone support beyond what host can provide
- CSV parser (that is #055, separate issue)

---

## Primary paths

- `std/json/mod.ark`
- `std/toml/mod.ark`
- `std/text/mod.ark`
- `std/time/mod.ark`
- `std/host/clock.ark`
- `tests/fixtures/` (stdlib fixtures for targeted families)

## Allowed adjacent paths

- `std/manifest.toml`
- `docs/stdlib/modules/` (json.md, toml.md, text.md, time.md)

---

## Upstream / Depends on

604 (contract honesty — must not add semantics while names are still misleading)

## Blocks

- #608 (docs/bench closeout)

---

## Acceptance

1. `std::json` parse rejects trailing garbage in a negative fixture
2. `std::toml` bounded subset is prominently documented with negative fixtures at boundaries
3. `std::text` distinguishes byte vs char operations in doc comments
4. `std::time` / `std::host::clock` split is explicit in docs and API surface

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python3 scripts/gen/generate-docs.py
```

---

## STOP_IF

- Do not implement full TOML 1.0 compliance
- Do not add Unicode normalization or timezone handling
- Do not implement CSV in this issue

---

## Close gate

Close when: all four families have updated docs, negative fixtures pass, and the
time/clock split is explicit and fixture-backed.