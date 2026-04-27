---
Status: open
Created: 2026-03-28
Updated: 2026-04-22
ID: 051
Track: stdlib
Depends on: 039, 040
Orchestration class: blocked-by-upstream
Orchestration upstream: #529
---

# std::time + std::random: 時刻・期間・乱数
**Blocks v3 exit**: yes

**Status note**: Blocker-free stdlib lane. This issue does not carry the #312 generic monomorphization blocker from #044.

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/051-std-time-random.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

時刻取得 (WASI clock)、期間計算、乱数生成を実装する。
ベンチマーク・テスト・一意 ID 生成・シャッフル等に必要。

## Operational lane — 2026-04-25

**BLOCKED:** This issue hit a STOP_IF during Wave 2 dispatch. The `#529` selfhost transition removed the `ark-wasm` emitter which contained WASI `clock_time_get` imports. The new pure-Ark emitter does not yet generate WASI P2 clock imports, blocking actual utilization of `std::time`. Execution is frozen until #529 or downstream emitter roadmap restores WASI import synthesis.

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
