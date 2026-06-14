---
Status: done
Created: 2026-03-28
Updated: 2026-06-14
Closed: 2026-06-14
ID: 073
Track: wasi-feature
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: True
---

## Closed — 2026-06-14

Core P1 syscall registry (`wasi_p1_registry.ark`) + 15-import surface in
`sections_imports.ark` (clock/args/environ/fd_seek/tell/fdstat). Gate:
`check-wasi-p1-surface.py`. Remaining 31 syscalls deferred.

## 受け入れ条件

- [x] Core syscall registry + T3 imports for clock/args/environ/fd I/O baseline
- [x] Fixtures: `wasi_clock.ark`, `wasi_random.ark`, `wasi_args.ark` in manifest
- Deferred: full 46-syscall table, errno helpers for all paths, std module coverage

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

## 参照

- `docs/spec/spec-WASI-0.1.0/OVERVIEW.md`
- `docs/spec/spec-WASI-0.1.0/preview1/witx/`
