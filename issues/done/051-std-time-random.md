---
Status: done
Created: 2026-03-28
Updated: 2026-06-15
Closed: 2026-06-15
ID: 41
Track: stdlib
Depends on: 039, 040
Orchestration class: done
Orchestration upstream: None
Blocks: none
Blocks v3 exit: no
Status note: Umbrella closed after child slices #661 (clock/random intrinsics) and #662 (duration typecheck).
---

## Close note — 2026-06-15

Umbrella rollup after Wave 1 child dispatch:

- **#661** — selfhost emitter handles `__intrinsic_clock_now`, `__intrinsic_clock_now_ms`, and
  `__intrinsic_random_i32`; T3 WASI P2 clock/random imports; `wasi_clock.ark` runs without trap.
- **#662** — selfhost typechecker i64 duration inference; `stdlib_time/monotonic.ark` and
  `duration.ark` pass.
- **std::random** — seeded xorshift32 surface unchanged and passing.

**Honest boundary:** `sleep_ms` is **not** implemented. Host clock reads live in
`std::host::clock` (`monotonic_now`, `now_ms`); pure duration math lives in `std::time`.
Blocking sleep requires `poll_oneoff` / async WASI work and remains out of this umbrella scope.

**Verification gate:** `scripts/check/gate-051-std-time-random.py`

---

# std::time + std::random: 時刻・期間・乱数

## Summary

時刻取得 (WASI clock)、期間計算、乱数生成を実装する。

## 子 issue

- [#661 std::time/host clock and random intrinsics (emitter)](661-std-time-clock-intrinsics-emitter.md)
- [#662 std::time duration helpers typecheck fix](662-std-time-duration-typecheck.md)

## 注意点

1. monotonic clock はプロセス間で比較不可
2. random は暗号学的安全性を保証しない
3. `sleep_ms` は未実装 — poll_oneoff 相当の将来 work
