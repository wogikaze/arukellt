---
Status: open
Created: 2026-03-28
Updated: 2026-05-17
ID: 41
Track: stdlib
Depends on: 039, 040
Orchestration class: partially-blocked
Orchestration upstream: None
Blocks v3 exit: yes
Status note: Partially blocked. Seeded std::random sub-surface is unblocked and working. std::time has a selfhost typechecker regression (i64 vs i32). host::clock and host::random intrinsics still blocked by missing emitter dispatch handlers.
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
BLOCKED: "The selfhost emitter (`src/compiler/emitter.ark`) lacks intrinsic dispatch handlers for `__intrinsic_clock_now`, `__intrinsic_clock_now_ms`, and `__intrinsic_random_i32`. The old Rust `ark-wasm` emitter handled these (commit 3f4bc5be) but was removed by #529. Additionally, the selfhost typechecker has a regression on i64 division inference affecting `std/time/mod.ark`. The deterministic `std::random` seeded surface (xorshift32 in `std/random/mod.ark`) is unblocked and working (not blocked by this issue)."
---

# std: ":time + std::random: 時刻・期間・乱数"

- `std: ":time` / WASI clock / sleep work is intentionally untouched in this slice."

## std: ":random"

pub fn duration_ms(start: "i64, end: i64) -> i64"
pub fn sleep_ms(ms: "i64)  // target-gated: WASI のみ"
pub fn random_i32_range(min: "i32, max: i32) -> i32"
pub fn shuffle<T>(v: Vec<T>) -> Vec<T>
pub fn seed(s: u64)  // seedable RNG for reproducibility
1. `std/time/time.ark`: WASI `clock_time_get` bridge
2. `std/random/random.ark`: "xorshift64 PRNG (deterministic, seedable)"
3. `ark-wasm/src/emit`: "WASI P2 `wasi:clocks/monotonic-clock` import"
- fixture: `stdlib_time/monotonic.ark`, `stdlib_time/duration.ark`,

## std::time + std::random: 時刻・期間・乱数

---

## Reopened by audit — 2026-04-03

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/051-std-time-random.md` — incorrect directory for an open issue.

## Summary

時刻取得 (WASI clock)、期間計算、乱数生成を実装する。
ベンチマーク・テスト・一意 ID 生成・シャッフル等に必要。

## Operational lane — 2026-04-25

**BLOCKED:** This issue hit a STOP_IF during Wave 2 dispatch. The `#529` selfhost transition removed the `ark-wasm` emitter which contained WASI `clock_time_get` imports. The new pure-Ark emitter does not yet generate WASI P2 clock imports, blocking actual utilization of `std::time`. Execution is frozen until #529 or downstream emitter roadmap restores WASI import synthesis.

## Assessment — 2026-05-17

### Current status: PARTIALLY UNBLOCKED, PARTIALLY BLOCKED

**Split assessment by sub-surface:**

#### 1. std::random (seeded deterministic) -- UNBLOCKED

`std/random/mod.ark` provides working seeded xorshift32 PRNG with `seeded_random`, `seeded_range`, `shuffle_i32`. These are pure Ark code using no intrinsics.
- `random_basic.ark` -- COMPILES AND RUNS correctly
- `random_range.ark` and `random_seed.ark` -- registered as `t3-run:` (expected to pass)
- **No emitter changes required.** This surface is ready for closure if the existing fixtures pass consistently.

#### 2. std::time (duration math) -- NEW TYPE BUG

`std/time/mod.ark` provides pure `duration_ms`, `duration_us`, `duration_ns` functions. The selfhost typechecker reports a regression: all three functions are declared as `-> i64` but the selfhost typechecker infers the body as `i32`.
- `stdlib_time/monotonic.ark` -- FAILS TO COMPILE with 3 type errors
- `stdlib_time/duration.ark` -- registered as `module-run:` (compile-only), may succeed
- **Root cause:** Selfhost typechecker regression in i64 division/if-expr typing (not in emitter)

#### 3. std::host::clock (intrinsic clock_now) -- STILL BLOCKED

`std/host/clock.ark` calls `__intrinsic_clock_now()` and `__intrinsic_clock_now_ms()`. These are registered in the Rust compiler's resolver (`crates/ark-resolve/src/bind.rs`) and typechecker (`crates/ark-typecheck/src/checker/builtins.rs`) but the selfhost pure-Ark emitter (`src/compiler/emitter.ark`) has NO handler for them. The fallback path emits `UNREACHABLE`, causing a runtime trap.
- `clock_random.ark` -- COMPILES but CRASHES at runtime (wasm `unreachable` trap)
- `wasi_clock.ark` -- COMPILES but CRASHES at runtime
- **Blocked until:** `__intrinsic_clock_now` and `__intrinsic_clock_now_ms` are handled in the selfhost emitter's intrinsic dispatch chain (lines 795-1447 of `src/compiler/emitter.ark`). The old Rust `ark-wasm` emitter had this in the T3 target (commits `3f4bc5be`, `35f2e7bb`). Equivalent logic needs porting to the pure-Ark emitter.

#### 4. std::host::random (intrinsic random_i32) -- STILL BLOCKED

Same situation as clock. `std/host/random.ark` calls `__intrinsic_random_i32()`. The emitter has no handler, causing runtime `UNREACHABLE`.
- **Blocked until:** `__intrinsic_random_i32` is handled in the selfhost emitter.

### Summary

| Subsurface | Status | Action needed |
|------------|--------|---------------|
| std::random (seeded) | Works | Close sub-surface, keep issue for other parts |
| std::time (duration) | Type bug | Fix selfhost typechecker i64 division inference |
| host::clock | Blocked | Add clock_now/clock_now_ms to emitter dispatch |
| host::random | Blocked | Add random_i32 to emitter dispatch |

Dependencies 039 and 040 are both DONE (verified). The blocker is now the selfhost emitter's missing intrinsic handlers, not WASI import synthesis per se.

## Partial progress — 2026-04-22

This slice is limited to the deterministic `std::random` surface:

- `std/random/mod.ark` now uses xorshift32-style seeded generation for reproducible outputs.
- `random_basic`, `random_range`, `random_seed`, and `shuffle` fixtures now pin exact seeded results and range edge behavior.
- `std::time` / WASI clock / sleep work is intentionally untouched in this slice.
- The `stdlib_time/duration.ark` fixture is now focused on `duration_ms(start, end)` only.

## 受け入れ条件

### std::time

```ark
pub fn now_unix_ms() -> i64
pub fn monotonic_now_ns() -> i64
pub fn duration_ms(start: i64, end: i64) -> i64
pub fn sleep_ms(ms: i64)  // target-gated: WASI のみ
```

### std::random

```ark
pub fn random_u32() -> u32
pub fn random_u64() -> u64
pub fn random_i32_range(min: i32, max: i32) -> i32
pub fn random_f64() -> f64  // 0.0 ..< 1.0
pub fn random_bool() -> bool
pub fn shuffle<T>(v: Vec<T>) -> Vec<T>
pub fn seed(s: u64)  // seedable RNG for reproducibility
```

## 実装タスク

1. `std/time/time.ark`: WASI `clock_time_get` bridge
2. `std/random/random.ark`: xorshift64 PRNG (deterministic, seedable)
3. `ark-wasm/src/emit`: WASI P2 `wasi:clocks/monotonic-clock` import
4. random_u32/random_u64 は WASI `random_get` で seed し、xorshift で生成
5. `seed()` 関数で再現可能テストを支援
6. `shuffle` は Fisher-Yates で実装

## 検証方法

- fixture: `stdlib_time/monotonic.ark`, `stdlib_time/duration.ark`,
  `stdlib_random/random_basic.ark`, `stdlib_random/random_range.ark`,
  `stdlib_random/random_seed.ark`, `stdlib_random/shuffle.ark`

## 完了条件

- 時刻取得が WASI 経由で動作する
- seeded RNG で同一シードから同一列を生成できる
- fixture 6 件以上 pass

## 注意点

1. monotonic clock はプロセス間で比較不可 — ドキュメントで明記
2. random は暗号学的安全性を保証しない — ドキュメントで明記
3. sleep は WASI target でのみ利用可能 — 他 target では compile error

## ドキュメント

- `docs/stdlib/time-random-reference.md`

## 未解決論点

1. DateTime 型 (年月日時分秒) を v3 に入れるか
2. CSPRNG (暗号学的安全な乱数) を v3 に含めるか (v4 送り推奨)
