---
Status: done
Created: 2026-03-31
Updated: 2026-06-13
Closed: 2026-06-13
ID: 295
Track: capability
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 15
---

# host API の run-time テストを拡充する

## Summary

clock, random, fs, env, process の host API に対する manifest 登録済み fixture が selfhost emitter 経由で実行・検証される。

## Delivered

- `src/compiler/wasm/call_host_time.ark` — `__intrinsic_clock_now` / `clock_now_ms` / `random_i32` dispatch
- `tests/fixtures/stdlib_io/clock_random.ark` — clock monotonic + random i32 range
- `tests/fixtures/stdlib_io/fs_read_error.ark` — fs read error
- `tests/fixtures/stdlib_env/env_basic.ark` — argv
- `tests/fixtures/host/process/exit_zero.ark` — `run:` + `t3-run:`
- `tests/fixtures/manifest.txt` — 上記を `run:` / `module-run:` / `t3-run:` 登録

## Acceptance（#295 スコープ）

- [x] clock: 2回呼び出しで単調増加（`clock_random.ark`, `module-run:stdlib_host/wasi_clock.ark`）
- [x] random: API 成功・i32 範囲（`clock_random.ark`）
- [x] fs: 存在しないファイル読み取りエラー（`fs_read_error.ark`）
- [x] env: arg_count / args（`env_basic.ark`）
- [x] process: exit(0) 正常終了（`host/process/exit_zero.ark`）
- [x] 全テストが CI harness（manifest）に登録され `verify quick` で実行される

## Out of scope（別 issue）

- **#051** — `stdlib::time` モジュール全体（i64 回帰、`monotonic.ark` 等の time 抽象）
- **http / sockets** — manifest に fixture はあるが本 issue の acceptance 対象外

## Verification

- `python scripts/manager.py verify quick` — 150/150 pass（clock_random, exit_zero, env_basic, fs_read_error 含む）

## Audit resolution — 2026-06-13

**Reopen reason addressed**: 6/12 は `call_host_time.ark` 未実装を根拠に reopen。現行 selfhost は clock/random を lower し manifest fixture が pass。

**#051 境界**: #295 は **host intrinsic + manifest fixture** の実行証拠。#051 は stdlib `time` モジュール API・型・追加 fixture の完了を追う。

**Evidence**: `call_host_time.ark`, `tests/fixtures/manifest.txt` entries for clock_random / exit_zero
