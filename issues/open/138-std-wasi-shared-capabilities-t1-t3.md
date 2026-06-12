---
Status: open
Created: 2026-03-29
Updated: 2026-06-12
ID: 138
Track: wasi-feature
Depends on: 137, 051
Orchestration class: blocked-by-upstream
Blocks v1 exit: False
Blocked by: "#051 std::time/clock intrinsics (reopened wave 1)"
---

## Reopened by audit — 2026-06-12 (Slice C)

**Reopen reason:** Never re-closed after 2026-04-03 reopen. Acceptance requires T1/T3 smoke for six `std::host::*` modules plus `verify-harness.sh`; `scripts/run/verify-harness.sh` deleted (#531), clock/random host intrinsics reopened as #051, only partial `stdlib_host/*` fixtures remain.

**Violated acceptance:** Items 3–5 (T1/T3 runtime fixtures for all six modules, execution records, verify-harness gate)

**Evidence files:**
- `tests/fixtures/manifest.txt` (partial `stdlib_host/wasi_{clock,random,args}.ark`; no full six-module matrix)
- `issues/open/051-std-time-random.md` (clock/random gap)
- Absent `scripts/run/verify-harness.sh`

**Follow-up split issue:** none (#051 tracks clock/random sub-surface)

---

# `std::host` 共通 capability (`stdio` / `fs` / `env` / `process` / `clock` / `random`) を T1/T3 両対応で実装

## Reopened by audit — 2026-04-03 (historical)

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/138-std-wasi-shared-capabilities-t1-t3.md` — incorrect directory for an open issue.

## Audit resolution — 2026-06-12

FD-01 Slice A review: frontmatter `Action` records a 2026-04 false-done move to `issues/open/`; file correctly remains under `issues/done/` after re-close verification.

**Evidence**: historical Rust-era completion superseded by selfhost-first migration (ADR-029); no active user-visible claim contradicted in current repo

**Classification**: `truly-done` (stale reopen metadata only).

## Summary

T1/T3 の両方で意味を共有できる host capability を `std::host::*` として提供する。
対象は `std::host::stdio`, `std::host::fs`, `std::host::env`, `std::host::process`, `std::host::clock`, `std::host::random` とし、
backend 差分は namespace ではなく target-specific lowering で吸収する。

## 受け入れ条件

1. 上記 6 module の public API が `std/manifest.toml` と `std/*.ark` に追加される
2. 各 module に doc comments を付け、`docs/stdlib` が自動生成で追従する
3. compile / runtime fixture を追加し、T1 (`wasm32-wasi-p1`) と T3 (`wasm32-wasi-p2`) の両方で smoke test が実行される
4. `arukellt run --target wasm32-wasi-p1 ...` と `arukellt run --target wasm32-wasi-p2 ...` の実行記録を verification に残す
5. `bash scripts/run/verify-harness.sh --quick` が status 0

## 実装タスク

1. `std::host::stdio`, `std::host::env`, `std::host::process` に stdio / args / env / exit 相当の facade を定義する
2. `std::host::fs` に read / write の最小 surface を定義する
3. `std::host::clock`, `std::host::random` を target-specific backend へ接続する
4. T1/T3 共通の runtime harness を追加し、実行結果を fixture と baseline に取り込む

## 参照

- `docs/adr/ADR-011-wasi-host-layering.md`
- `docs/migration/t1-to-t3.md`
