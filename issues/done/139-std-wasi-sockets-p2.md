---
Status: done
Created: 2026-03-29
Updated: 2026-06-15
Closed: 2026-06-15
ID: 139
Depends on: 074, 137
Track: wasi-feature
Orchestration class: done
Orchestration upstream: None
Blocks: none
Blocks v{N}: none
Status note: Umbrella closed after child slices #657 (connect+read) and #658 (listen+accept).
---

## Close note — 2026-06-15

WASI Preview 2 sockets capability is available via `std::host::sockets` with T1 rejection and
T3 runtime smoke on wasmtime / host-linker:

- **#657** — TCP connect, read, write client path (`connect_read_write.ark`, `gate-657`).
- **#658** — TCP listen, accept server path (`listen_accept.ark`, `gate-658`).

**Verification gate:** `scripts/check/gate-139-wasi-p2-sockets-umbrella.py`

---

# WASI P2: `std::host::sockets` facade と T3 実行検証

## Summary

WASI Preview 2 の sockets capability を `std::host::sockets` として提供する。
ユーザー向け API は capability 名で固定し、P2 / Component 実装差分は backend に閉じ込める。

## 受け入れ条件

1. [x] `std::host::sockets` の最小 public API が `std/manifest.toml` と `std/*.ark` に定義される
2. [x] T1 で `use std::host::sockets` した場合は専用 diagnostics で compile-time error になる
3. [x] T3 では wasmtime 等の P2 対応ランタイム上で実際に socket I/O が動作する
4. [x] compile fixtures, runtime fixtures, docs examples が追加される
5. [x] `python3 scripts/manager.py verify quick` が status 0

## 子 issue

- [#657 WASI P2 sockets: connect and read/write](657-std-wasi-sockets-connect-read.md)
- [#658 WASI P2 sockets: listen and accept](658-std-wasi-sockets-listen-accept.md)

## 参照

- `docs/adr/ADR-011-wasi-host-layering.md`
- `issues/done/074-wasi-p2-native-component.md`
