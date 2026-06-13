---
Status: done
Created: 2026-03-30
Updated: 2026-06-13
ID: 236
Track: main
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: yes
---

# CLI 起動契約を明確化する（LSP 起動方法・バージョン検出・stdio 扱い）

## Summary

`arukellt lsp` / `arukellt debug-adapter` の stdio 起動契約を selfhost CLI と拡張機能で満たし、文書化した。stdin transport は #634 で完了。

## Delivered

- [`src/compiler/main/editor.ark`](../../src/compiler/main/editor.ark) — stdin JSON-RPC と script リプレイ
- [`docs/cli-startup-contract.md`](../../docs/cli-startup-contract.md) — version 形式、stdout/stderr 分離
- [`extensions/arukellt-all-in-one/src/extension.js`](../../extensions/arukellt-all-in-one/src/extension.js) — `TransportKind.stdio`
- [`scripts/check/check-lsp-lifecycle.py`](../../scripts/check/check-lsp-lifecycle.py) — file-arg と stdin stdio の両経路

## Acceptance

- [x] `arukellt lsp` の起動引数・stdio 使用方法が文書化されている
- [x] `arukellt --version` が機械可読なフォーマットでバージョンを返す（`arukellt 0.1.0`、exit 0）
- [x] stderr と stdout の用途が明確に分離されている（診断 vs プロトコル）
- [x] 拡張機能が CLI なしで起動した場合に actionable なエラーを出す

## Verification

- `python3 scripts/check/check-lsp-lifecycle.py` — 11/11 pass（stdio + script）
- `python scripts/manager.py verify quick` — 150/150 pass

## Audit resolution — 2026-06-13

**Reopen reason addressed**: 6/12 audit の「script-only LSP」は #634（stdin `run_session`）で解消済み。`check-lsp-lifecycle.py` が両経路を検証。

**Evidence**: `editor.ark` `cmd_lsp`, `docs/cli-startup-contract.md`, lifecycle gate
