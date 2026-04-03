# WASI P1: 全46 syscall 対応 (clock / random / proc_exit / fd_seek 等)

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 073
**Depends on**: —
**Track**: wasi-feature
**Blocks v4 exit**: yes

**Status note**: WASI feature — deferred to v5+. Requires WASI P2 runtime maturity.


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/073-wasi-p1-full-syscalls.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

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
