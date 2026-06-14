---
Status: open
Created: 2026-06-15
ID: 662
Track: stdlib
Parent: 051
Depends on: 039, 040
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v3 exit: no
Status note: Child of #051 — selfhost typechecker i64 duration inference fix.
---

# 662 — std::time duration helpers typecheck fix

## Summary

Fix selfhost typechecker regression where `std/time/mod.ark` duration helpers (`duration_ms`,
`duration_us`, `duration_ns`) infer `i32` bodies despite `-> i64` signatures. Unblocks
`tests/fixtures/stdlib_time/monotonic.ark`.

## Parent

Umbrella: [#051 std::time + std::random](051-std-time-random.md)

## Acceptance

- [ ] `duration_ms`, `duration_us`, `duration_ns` compile without E0200 i64 vs i32 errors
- [ ] `tests/fixtures/stdlib_time/monotonic.ark` compiles and runs
- [ ] `tests/fixtures/stdlib_time/duration.ark` passes
- [ ] No regression in other i64 arithmetic fixtures
- [ ] Independent of #661 (parallel dispatch OK)
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/051-std-time-random.md`
- `std/time/mod.ark`
- `src/compiler/typechecker.ark`
