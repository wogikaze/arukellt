---
Status: open
Created: 2026-03-31
Updated: 2026-06-12
Closed: 2026-06-12
ID: 295
Track: capability
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 15
---

- `tests/fixtures/stdlib_io/clock_random.ark`: clock + random の最小テスト
- `tests/fixtures/stdlib_io/fs_read_write.ark`: fs の読み書きテスト
- `tests/fixtures/stdlib_env/env_basic.ark`: env の最小テスト
- process: ":exit の正常系テストがない"
- [x] clock: 2回呼び出しで単調増加を確認する fixture
- [x] random: API 呼び出しが成功し、返り値が i32 範囲内であることを確認する fixture（非決定性に依存しない）
- [x] fs: 存在しないファイルの読み取りエラーを確認する fixture（既存 `fs_read_error.ark` で可）
- [x] env: arg_count / args の引数受け渡しを確認する fixture
- [x] process: "exit(0) の正常終了を確認する fixture"

# host API の run-time テストを拡充する

## Reopened by audit — 2026-06-12 (slice D)

**Classification**: `must-reopen` / `acceptance-not-actually-met`

**Reopen reason**: Acceptance requires clock monotonic and random host API fixtures to pass. Selfhost emitter has no handlers for `__intrinsic_clock_now`, `__intrinsic_clock_now_ms`, or `__intrinsic_random_i32` (same root cause as reopened #051).

**Repo evidence**:

- `rg '__intrinsic_clock|__intrinsic_random' src/compiler/wasm/` returns no dispatch handlers.
- `tests/fixtures/stdlib_io/clock_random.ark` and `tests/fixtures/stdlib_time/monotonic.ark` call `std::host::clock` / random surfaces that cannot lower on the selfhost path.
- `tests/fixtures/manifest.txt` registers these as `run:` / `t3-run:` entries, but close evidence must be emitter-backed execution, not fixture file presence alone.

**Violated acceptance**: clock monotonic fixture, random API fixture (partial credit: env argv + fs fixtures may still be valid).

**Evidence files**: `src/compiler/wasm/call_host_io.ark`, `tests/fixtures/stdlib_io/clock_random.ark`, `tests/fixtures/stdlib_time/monotonic.ark`, `issues/open/051-std-time-random.md`

**Follow-up split**: consider splitting env/fs/process (potentially met) from clock/random (blocked on #051)

---

# host API の run-time テストを拡充する

## Summary

使用可能な host API に対する CI テストが不十分。clock, random, fs, env の各モジュールで、正常系・異常系の fixture を追加する。

## Current state

- `tests/fixtures/stdlib_io/clock_random.ark`: clock + random の最小テスト
- `tests/fixtures/stdlib_io/fs_read_write.ark`: fs の読み書きテスト
- `tests/fixtures/stdlib_env/env_basic.ark`: env の最小テスト
- process::exit の正常系テストがない

## Acceptance

- [x] clock: 2回呼び出しで単調増加を確認する fixture
- [x] random: API 呼び出しが成功し、返り値が i32 範囲内であることを確認する fixture（非決定性に依存しない）
- [x] fs: 存在しないファイルの読み取りエラーを確認する fixture（既存 `fs_read_error.ark` で可）
- [x] env: arg_count / args の引数受け渡しを確認する fixture
- [ ] process: exit(0) の正常終了を確認する fixture
- [ ] 全テストが CI harness に登録される

## References

- `tests/fixtures/stdlib_io/`
- `tests/fixtures/stdlib_env/`
- `tests/fixtures/manifest.txt`
