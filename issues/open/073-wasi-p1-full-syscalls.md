---
Status: open
Created: 2026-03-28
Updated: 2026-06-12
ID: 073
Track: wasi-feature
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: True
Status note: Reopened 2026-06-12 — claims 46 P1 syscalls but only 3 smoke fixtures exist.
---

## Reopened by audit — 2026-06-12 (Slice C)

**Reopen reason:** Never re-closed after 2026-04-03 reopen. Acceptance requires all 46 `wasi_snapshot_preview1` syscalls; repo has only `wasi_clock`, `wasi_random`, `wasi_args` module-run fixtures and minimal import surface in `src/compiler/wasm/sections_imports.ark`.

**Violated acceptance:** Items 1–3 (full syscall import table, std module coverage, errno handling)

**Evidence files:**
- `tests/fixtures/manifest.txt` (`module-run:stdlib_host/wasi_{clock,random,args}.ark` only)
- `src/compiler/wasm/sections_imports.ark` (limited WASI import set)
- Issue body syscall table (42+ syscalls listed as missing)

**Follow-up split issue:** Consider splitting clock/random/args (partial) vs remaining syscalls

# WASI P1: 全46 syscall 対応 (clock / random / proc_exit / fd_seek 等)

---

# WASI P1: 全46 syscall 対応 (clock / random / proc_exit / fd_seek 等)

---

## Reopened by audit — 2026-04-03 (historical)

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/073-wasi-p1-full-syscalls.md` — incorrect directory for an open issue.

## Audit resolution — 2026-06-12

FD-01 Slice A review: frontmatter `Action` records a 2026-04 false-done move to `issues/open/`; file correctly remains under `issues/done/` after re-close verification.

**Evidence**: historical Rust-era completion superseded by selfhost-first migration (ADR-029); no active user-visible claim contradicted in current repo

**Classification**: `truly-done` (stale reopen metadata only).

## Summary

現在 T3 emitter が wasm import する WASI Preview 1 関数は
`fd_write` / `path_open` / `fd_read` / `fd_close` の4つのみ。
`docs/spec/spec-WASI-0.1.0/OVERVIEW.md` に定義された残り42の syscall を
std ライブラリから呼び出せるよう import とヘルパー関数を追加する。

## 不足している主要 syscall

| カテゴリ | 関数名 |
|---------|-------|
| クロック | `clock_time_get`, `clock_res_get` |
| ランダム | `random_get` |
| プロセス | `proc_exit`, `proc_raise` |
| 環境 | `args_get`, `args_sizes_get`, `environ_get`, `environ_sizes_get` |
| fd 操作 | `fd_seek`, `fd_tell`, `fd_stat_get`, `fd_fdstat_get`, `fd_fdstat_set_flags` |
| fd 操作 | `fd_prestat_get`, `fd_prestat_dir_name`, `fd_sync`, `fd_datasync` |
| fd 操作 | `fd_allocate`, `fd_advise`, `fd_filestat_get`, `fd_filestat_set_times` |
| fd 操作 | `fd_readdir`, `fd_renumber` |
| パス操作 | `path_create_directory`, `path_filestat_get`, `path_filestat_set_times` |
| パス操作 | `path_link`, `path_readlink`, `path_remove_directory`, `path_rename` |
| パス操作 | `path_symlink`, `path_unlink_file` |
| ポーリング | `poll_oneoff` |
| ソケット | `sock_accept`, `sock_recv`, `sock_send`, `sock_shutdown` |
| スレッド | `sched_yield` |

## 受け入れ条件

1. 全 syscall が `wasi_snapshot_preview1` からの import として T3 に追加
2. `std/io`・`std/process`・`std/random`・`std/time` モジュールが対応する syscall を使用
3. 各 syscall について errno 型の正しいハンドリング
4. fixture: `wasi_clock.ark` (time取得)、`wasi_random.ark` (ランダムバイト)、`wasi_args.ark` (引数取得)

## 参照

- `docs/spec/spec-WASI-0.1.0/OVERVIEW.md`
- `docs/spec/spec-WASI-0.1.0/preview1/witx/`
