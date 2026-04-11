# Wasm Sign Extension Ops: i32.extend8_s / i32.extend16_s / i64 版

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 067
**Depends on**: —
**Track**: wasm-feature
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/067-wasm-sign-extension-ops.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

WebAssembly Sign Extension Ops 提案 (`docs/spec/spec-1.0.0/proposals/sign-extension-ops/Overview.md`) の
`i32.extend8_s`・`i32.extend16_s`・`i64.extend8_s`・`i64.extend16_s`・`i64.extend32_s` を活用する。
現在 `i8` / `i16` の sign extension は `shl` + `shr_s` の2命令で実装しているが、
専用命令に置き換えることでバイナリサイズを削減し、JIT の peephole 最適化を促進する。

## 受け入れ条件

1. `i8_to_i32` / `i16_to_i32` (符号付き変換) が `i32.extend8_s` / `i32.extend16_s` を emit
2. `i64` の narrow → wide 符号付き変換も専用命令を使用
3. MIR の `UnaryOp::SignExtend8` 等を追加して T3 で対応命令に map
4. 変換後のバイナリサイズが減少していることを `wc -c` で確認

## 参照

- `docs/spec/spec-1.0.0/proposals/sign-extension-ops/Overview.md`
