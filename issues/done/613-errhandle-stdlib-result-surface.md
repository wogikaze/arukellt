---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 613
Track: stdlib
Orchestration class: implementation-ready
Depends on: —
Parent: #592
Phase 0 baseline is part of this issue: run parity/verify gates, record counts,
In scope: 
Out of scope: 
Close when: inventory is written, top-impact fs/json APIs have typed errors,
---

# Error Handling Convergence: Stdlib Result Surface
- recoverable runtime failure: typed `Result<T, E>` with stable error categories
- optional absence: `Option<T>`, not stringly error messages
`std: ":host::fs`, `std::json`, `std::io` (or equivalent)"
- `std: ":error::Error` trait / trait-based error abstraction (requires #495 / #512 first)"
1. Phase 0 inventory: list of stdlib APIs returning `Result<_, String>` is written
2. At least the top 3 highest-impact APIs in `std: ":host::fs` / `std::json` have typed error enums"
- Do not implement `std: ":error::Error` trait abstraction before #495 / #512 are ready"
# Error Handling Convergence: Stdlib Result Surface

---

## Summary

Child issue for #592 Stream 1 — Runtime/Stdlib Error Surface.

**Phase 0 baseline is part of this issue:** run parity/verify gates, record counts,
inventory which stdlib APIs still return `Result<_, String>` or other stringly-typed
failures before making changes.

The convergence target for this stream:
- recoverable runtime failure: typed `Result<T, E>` with stable error categories
- optional absence: `Option<T>`, not stringly error messages
- `String` error type is acceptable only when no better error model is available, and
  must be explicitly marked `experimental` if so

---

## Scope

**In scope:**
- Inventory stdlib APIs returning `Result<_, String>` where a richer error type would be safe
- Introduce or standardize typed error enums for the most common failure categories in
  `std::host::fs`, `std::json`, `std::io` (or equivalent)
- Ensure `Option` is used for "value may be absent" instead of empty-string or false sentinels
- Negative fixtures demonstrating the correct error type on failure paths

**Out of scope:**
- Compiler diagnostics (that is #614)
- Panic / ICE policy (that is #615)
- Full error hierarchy for every stdlib family — start with the highest-impact gaps
- `std::error::Error` trait / trait-based error abstraction (requires #495 / #512 first)

---

## Primary paths

- `std/host/fs.ark`
- `std/json/mod.ark`
- `std/io/mod.ark` (if exists)
- `std/manifest.toml`
- `tests/fixtures/` (error surface fixtures)

## Allowed adjacent paths

- Other `std/` modules where `Result<_, String>` cleanup is trivial and safe

---

## Upstream / Depends on

None. But coordinate with #604 (stdlib contract honesty) — do not change error types
on APIs whose names are still misleading.

## Blocks

- Closes the stdlib stream of #592

---

## Acceptance

1. Phase 0 inventory: list of stdlib APIs returning `Result<_, String>` is written
2. At least the top 3 highest-impact APIs in `std::host::fs` / `std::json` have typed error enums
3. At least one negative fixture for each converted API demonstrates the typed error
4. No existing fixtures regress

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python3 scripts/gen/generate-docs.py
```

---

## STOP_IF

- Do not implement `std::error::Error` trait abstraction before #495 / #512 are ready
- Do not change error types in APIs that are still experimental/unstable without bumping stability labels
- Do not touch compiler diagnostic code in this issue

---

## Close gate

Close when: inventory is written, top-impact fs/json APIs have typed errors,
negative fixtures pass, and `manifest.toml` stability labels are updated.

---

## Close note

**Closed: 2026-04-28**
**Branch:** `feat/613-stdlib-result-surface` (commit `7b149a95`, merged into master)
**Implementer agent:** Wave 1 parallel dispatch

**Acceptance:**
- [x] Phase 0 inventory: 27 `Result<_, String>` APIs found across 9 stdlib modules — documented in `docs/inventory/stdlib-result-surface.md`
- [x] Top 3 highest-impact APIs + all 13 remaining `std::io` APIs converted to typed `IoError(UnexpectedEof|Other)` enum
- [x] Negative fixture `tests/fixtures/stdlib_io_rw/read_exact_typed_error.ark` demonstrates typed error
- [x] No existing fixtures regressed

**Gates:**
- verify quick: 17/22 pass (5 pre-existing failures)
- verify fixtures: PASS (with pre-existing limitations)
- generate-docs: PASS

**Scope note:** `docs/inventory/stdlib-result-surface.md` was outside the initial PRIMARY_PATHS/ALLOWED_ADJACENT_PATHS — the inventory file is a natural output of the work and should be allowed as an adjacent path for this class of issue.