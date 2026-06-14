---
Status: open
Created: 2026-06-15
ID: 661
Track: stdlib
Parent: 051
Depends on: 039, 040
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v3 exit: no
Status note: Child of #051 — `__intrinsic_clock_now*` and `__intrinsic_random_i32` emitter dispatch.
---

# 661 — std::time/host clock and random intrinsics (emitter)

## Summary

Add selfhost emitter intrinsic handlers for `__intrinsic_clock_now`, `__intrinsic_clock_now_ms`,
and `__intrinsic_random_i32`. Port WASI P2 clock/random import synthesis from retired Rust emitter.

## Parent

Umbrella: [#051 std::time + std::random](051-std-time-random.md)

## Acceptance

- [ ] `__intrinsic_clock_now` and `__intrinsic_clock_now_ms` handled in `src/compiler/emitter.ark`
- [ ] `__intrinsic_random_i32` handled in emitter intrinsic dispatch
- [ ] WASI P2 `wasi:clocks/*` and random imports emitted for T3 target
- [ ] `clock_random.ark` and `wasi_clock.ark` fixtures run without `unreachable` trap
- [ ] `now_unix_ms` / `monotonic_now_ns` acceptance paths work at runtime
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/051-std-time-random.md`
- `std/host/clock.ark`, `std/host/random.ark`
- `src/compiler/emitter.ark`
