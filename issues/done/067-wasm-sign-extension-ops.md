---
Status: done
Created: 2026-03-28
Updated: 2026-06-14
Closed: 2026-06-14
ID: 067
Track: wasm-feature
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: False
---

## Closed — 2026-06-14

Sign-extension helpers + `i8_to_i32`/`i16_to_i32` lowering. Gate: `check-wasm-micro-features.py`.

# Wasm Sign Extension Ops: i32.extend8_s / i32.extend16_s / i64 版

## Summary

WebAssembly Sign Extension Ops 提案 (`docs/spec/spec-1.0.0/proposals/sign-extension-ops/Overview.md`) の
`i32.extend8_s`・`i32.extend16_s`・`i64.extend8_s`・`i64.extend16_s`・`i64.extend32_s` を活用する。
現在 `i8` / `i16` の sign extension は `shl` + `shr_s` の2命令で実装しているが、
専用命令に置き換えることでバイナリサイズを削減し、JIT の peephole 最適化を促進する。

## 受け入れ条件

- [x] `i8_to_i32` / `i16_to_i32` が `i32.extend8_s` / `i32.extend16_s` を emit (`helpers_numeric_sign_extend.ark`)
- [x] `tests/fixtures/scalar/sign_extend.ark` registered in manifest
- Deferred: i64 sign-ext ops, MIR `UnaryOp::SignExtend*`, binary size benchmark

## 参照

- `docs/spec/spec-1.0.0/proposals/sign-extension-ops/Overview.md`
