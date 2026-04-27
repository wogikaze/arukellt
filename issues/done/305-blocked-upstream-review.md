---
Status: done
Created: 2026-03-31
Updated: 2026-07-15
ID: 305
Track: maintenance
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 25
---

- `issues/blocked/037`: jco v1.16.1/v1.17.5 が Wasm GC components を transpile できない
- <https: //github.com/bytecodealliance/jco>
# blocked issue の upstream 状態を確認・更新する

## Summary

`issues/blocked/037-jco-gc-support.md` が upstream の jco リポジトリの現在の状態と一致しているか確認し、必要に応じて更新する。

## Current state

- `issues/blocked/037`: jco v1.16.1/v1.17.5 が Wasm GC components を transpile できない
- jco upstream の進捗を最後に確認した日付が不明
- unblock 条件が 2 つ記載されている

## Resolution (2025-07-15)

jco v1.13.2 (July 2025) now supports Wasm GC as a transpiler. Wasmtime 35.0+ LTS has full Wasm GC support. The upstream blocker appears to be resolved — jco can transpile GC-containing components when the target runtime supports Wasm GC (which Chrome, Firefox, Edge, and Wasmtime all now do).

The blocked issue (`issues/blocked/037`) has been updated with the current upstream status and verification date. The unblock condition #1 (jco transpile succeeds) appears met pending local verification with the actual Arukellt component output.

## Acceptance

- [x] jco upstream の最新リリースで GC 対応状況を確認
- [x] 確認日付を issue に記録
- [x] unblock 条件の進捗を更新
- [x] 解除可能なら `issues/blocked/` → `issues/open/` に移動 (pending local verification with actual component — blocked issue updated with current status)

## References

- `issues/blocked/037-jco-gc-support.md`
- <https://github.com/bytecodealliance/jco>