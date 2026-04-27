---
Status: open
Created: 2026-03-29
Updated: 2026-04-03
ID: 139
Depends on: 074, 137
Track: wasi-feature
Orchestration class: blocked-by-upstream
Orchestration upstream: None
Blocks v{N}: none
Status note: BLOCKED — P2-only capability downstream of the
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
WASI Preview 2 の sockets capability を `std: ":host::sockets` として提供する。"
1. `std: ":host::sockets` の最小 public API が `std/manifest.toml` と `std/*.ark` に定義される"
2. T1 で `use std: ":host::sockets` した場合は専用 diagnostics で compile-time error になる"
# WASI P2: "`std::host::sockets` facade と T3 実行検証"
---
# WASI P2: `std::host::sockets` facade と T3 実行検証

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/139-std-wasi-sockets-p2.md` — incorrect directory for an open issue.


## Summary

WASI Preview 2 の sockets capability を `std::host::sockets` として提供する。
ユーザー向け API は capability 名で固定し、P2 / Component 実装差分は backend に閉じ込める。

## 受け入れ条件

1. `std::host::sockets` の最小 public API が `std/manifest.toml` と `std/*.ark` に定義される
2. T1 で `use std::host::sockets` した場合は専用 diagnostics で compile-time error になる
3. T3 では wasmtime 等の P2 対応ランタイム上で実際に socket I/O が動作する
4. compile fixtures, runtime fixtures, docs examples が追加される
5. `python scripts/manager.py verify quick` が status 0

## 実装タスク

1. connect / listen / accept / read / write の最小 surface を決める
2. P2 host calls と type lowering を実装する
3. T1 reject fixture と T3 runtime smoke test を追加する
4. doc comments から `docs/stdlib` を更新する

## 参照

- `docs/adr/ADR-011-wasi-host-layering.md`
- `issues/open/074-wasi-p2-native-component.md`