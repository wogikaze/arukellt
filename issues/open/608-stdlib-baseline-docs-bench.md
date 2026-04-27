---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 608
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Stdlib Baseline: Docs, Verification, and Benchmark Closeout
**Parent**: #590
**Depends on**: 604, 605, 606, 607
**Track**: stdlib / docs
**Orchestration class**: blocked-by-upstream

---

## Summary

Child issue for #590 Phase 5 — Docs / Verification / Benchmark Closeout.

After contract honesty (#604), host platform (#605), structured data (#606), and hash
hardening (#607), this issue makes the stdlib readable and governable by ensuring
generated docs are trustworthy, benchmark coverage follows the fixed families, and no
stale progress surface contradicts `std/manifest.toml`.

---

## Scope

**In scope:**
- Verify all targeted modules have real module doc comments (no `_No module doc comment yet_`)
- Verify availability, stability, and deprecation are visible in generated docs
- Verify examples match current behavior (example smoke-check fixtures)
- Ensure `mise bench` covers: file I/O (#543 tie-in), parser/text builder hot paths (#520 tie-in),
  hash-family occupancy / collision / regression measurements
- Remove or update any hand-maintained progress boards that contradict `std/manifest.toml`

**Out of scope:**
- Implementing any stdlib features — those are #604-#607
- Full batteries-included breadth expansion

---

## Primary paths

- `docs/stdlib/modules/` (all targeted families)
- `std/manifest.toml`
- `benchmarks/` (hash, file I/O, text/parser hot paths)

---

## Upstream / Depends on

604, 605, 606, 607 (all implementation issues must be complete)

## Blocks

None (this closes the #590 umbrella)

---

## Acceptance

1. `python3 scripts/gen/generate-docs.py` completes with no `_No module doc comment yet_` entries
   for targeted families
2. `mise bench` covers at least: fs read/write, json parse, text operations, hash insert/get
3. No hand-maintained progress board contradicts `std/manifest.toml` stability labels

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
mise bench
python3 scripts/gen/generate-docs.py
python scripts/manager.py docs check
```

---

## STOP_IF

- Do not add new stdlib APIs — only verify and close the docs/bench gap
- Do not expand benchmark scope beyond the targeted families

---

## Close gate

This issue closes the #590 umbrella. Close when all targeted modules have real docs,
benchmark coverage exists for the major hot paths, and `manifest.toml` is authoritative.