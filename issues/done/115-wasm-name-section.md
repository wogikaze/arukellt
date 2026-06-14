---
Status: done
Created: 2026-03-28
Updated: 2026-06-14
Closed: 2026-06-14
ID: 115
Track: wasm-quality
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: False
---

## Closed — 2026-06-14

`sections_name.ark` emits empty `name` custom section at O0/O1 (skipped at O2+).
Gate: `check-wasm-micro-features.py`. Function/local name population deferred.

# Wasm Name Section: デバッグ用関数名・ローカル名セクション生成

## Summary

生成する Wasm バイナリに Name Section (custom section `name`) を追加し、
wasmtime のスタックトレースや `wasm-objdump` でのデバッグ体験を改善する。
`--opt-level 0` では名前情報を完全に含め、`--opt-level 2` では省略可能とする。

## 受け入れ条件

- [x] T3 emitter が `name` custom section を生成（O2+ では省略）
- Deferred: Ark 関数名/ローカル名の投入、wasmtime スタックトレース検証、`--strip-debug`

## 参照

- WebAssembly binary format §custom section
