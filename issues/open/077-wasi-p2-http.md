---
Status: open
Created: 2026-03-28
Updated: 2026-04-03
ID: 63
Depends on: 074, 137
Track: wasi-feature
Orchestration class: blocked-by-upstream
Orchestration upstream: #74
Blocks v{N}: none
Status note: BLOCKED — downstream of the #074 WASI P2 native parent gate. Runtime maturity is not the active blocker.
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
WASI Preview 2 の `wasi: "http/incoming-handler` と `wasi:http/outgoing-handler` を"
---

# WASI P2: "`std::host::http` facade と runtime 検証"
`std: ":host::http` として提供する。"
1. `std: ":host::http` に request / response / headers / body streaming の最小 API を追加する"
2. T1 で `use std: ":host::http` した場合は専用 diagnostics で compile-time error になる"
3. Arukellt プログラムが `wasi: http/proxy` world として HTTP サーバになれる
2. `wasi: http` binding と host lowering を backend に追加する
- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §wasi: http
# WASI P2: `std::host::http` facade と runtime 検証

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/077-wasi-p2-http.md` — incorrect directory for an open issue.


## Summary

WASI Preview 2 の `wasi:http/incoming-handler` と `wasi:http/outgoing-handler` を
`std::host::http` として提供する。
HTTP サーバ (incoming-handler world をエクスポート) と
HTTP クライアント (outgoing-handler をインポート) の両方を capability-based facade に載せる。

## 受け入れ条件

1. `std::host::http` に request / response / headers / body streaming の最小 API を追加する
2. T1 で `use std::host::http` した場合は専用 diagnostics で compile-time error になる
3. Arukellt プログラムが `wasi:http/proxy` world として HTTP サーバになれる
4. compile fixtures, runtime fixtures, docs examples が追加される
5. wasmtime (`wasi-http` feature) 上の T3 実行で HTTP client / server の両方を確認する

## 実装タスク

1. request / response / header map / body stream の public surface を設計する
2. `wasi:http` binding と host lowering を backend に追加する
3. T1 reject fixture と T3 runtime smoke test を追加する
4. doc comments から `docs/stdlib` を更新する

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §wasi:http
- `docs/spec/spec-WASI-0.2.10/proposals/wasi-http/`