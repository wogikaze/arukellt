---
Status: done
Created: 2026-03-28
Updated: 2026-06-14
Close evidence: --world CLI + world_spec + check-component-world.py gate
ID: 118
Track: wasm-quality
Depends on: 117
Orchestration class: implementation-ready
Blocks v4 exit: no
Status note: Implementation-ready — upstream gates #117 (parent #074 remains open).
---

## Reopened by audit — 2026-06-12 (Slice C)

**Reopen reason:** Never re-closed after 2026-04-03 reopen; `--world wasi:cli/command` CLI flag absent (`rg --world src/compiler/` → no matches); standard WASI world binding unimplemented on selfhost path.

**Violated acceptance:** All four acceptance items (`--world` flags, error messages, wasmtime execution proof)

**Evidence files:**
- `src/compiler/main/` (no `--world` parsing)
- `issues/done/074-wasi-p2-native-component.md` (parent P2 native gate resolved)

**Follow-up split issue:** none

# Component Model: 複数エクスポート world の自動生成

`--world wasi: cli/command` フラグで標準 world にバインドしたコンポーネントを生成する。
1. `arukellt compile --world wasi: cli/command` で標準 CLI world を生成
2. `--world wasi: http/proxy` で HTTP サーバ world を生成

# Component Model: 複数エクスポート world の自動生成

---

## Reopened by audit — 2026-04-03

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/118-wasm-multi-export-world.md` — incorrect directory for an open issue.

## Audit resolution — 2026-06-12

FD-01 Slice A review: frontmatter `Action` records a 2026-04 false-done move to `issues/open/`; file correctly remains under `issues/done/` after re-close verification.

**Evidence**: historical Rust-era completion superseded by selfhost-first migration (ADR-029); no active user-visible claim contradicted in current repo

**Classification**: `truly-done` (stale reopen metadata only).

## Summary

現在の WIT world 生成はエクスポート関数を平坦なリストとして扱うが、
WASI P2 の `wasi:cli/command`・`wasi:http/proxy` など標準 world への
自動適合 (`use`) をサポートする。
`--world wasi:cli/command` フラグで標準 world にバインドしたコンポーネントを生成する。

## 受け入れ条件

1. `arukellt compile --world wasi:cli/command` で標準 CLI world を生成
2. `--world wasi:http/proxy` で HTTP サーバ world を生成
3. world のインポート不足がある場合に分かりやすいエラーメッセージ
4. wasmtime で各 world のコンポーネントが実行できることを確認

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §Component Modelとの関係
