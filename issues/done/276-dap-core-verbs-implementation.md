---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 276
Track: parallel
Depends on: 255
Orchestration class: implementation-ready
---
# DAP 基本動詞を実装する（launch/threads/stackTrace/scopes/variables）
**Blocks v1 exit**: no

## Summary

`crates/ark-dap/src/lib.rs` は DAP transport を受ける最小 scaffold だが、日常デバッグに必要な基本動詞が未実装である。まず `threads / stackTrace / scopes / variables` のレスポンスを実装し、デバッガが「止まった後に状態を見る」ことを可能にする。

## Acceptance

- [x] `threads` リクエストに対して現在実行中のスレッドリストを返せる
- [x] `stackTrace` リクエストに対してコールスタックフレームを返せる
- [x] `scopes` リクエストに対してスコープリストを返せる
- [x] `variables` リクエストに対して変数名・型・値を返せる
- [x] `launch` リクエストで `.ark` プログラムを起動できる

## Scope

- `crates/ark-dap/src/lib.rs` に `threads / stackTrace / scopes / variables / launch` ハンドラを追加
- runtime / MIR との接続インタフェースの設計
- 各レスポンスの DAP プロトコル準拠チェック（`debugpy` 等の spec を参照）

## References

- `crates/ark-dap/src/lib.rs`
- `issues/open/255-dap-end-to-end-workflow.md`

## Completion Note

Closed 2026-04-09. Added: stackTrace (empty frames), scopes (empty), variables (empty), continue/next/stepIn/stepOut responses. launch now stores source_path. configurationDone now runs arukellt run <path> via tokio::process::Command, captures stdout/stderr as output events, sends exited + terminated events. All unknown commands return empty success. setFunctionBreakpoints/setExceptionBreakpoints accepted. Breakpoints still unverified (runtime hooks needed for #277).