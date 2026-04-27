---
Status: done
Created: 2026-03-28
Updated: 2026-04-15
ID: 066
Track: wasm-feature
Depends on: —
Orchestration class: implementation-ready
Orchestration upstream: —
---

# Wasm Bulk Memory: memory.copy / memory.fill / table.copy フル対応
**Blocks v4 exit**: no

**Status note**: Implemented in T3 (WasmGC) and T1 emitters; runtime.rs explicitly enables bulk_memory; std/wasm::memory_copy + memory_fill call actual intrinsics; fixture tests/fixtures/stdlib_wasm/memory_ops.ark verified.

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/066-wasm-bulk-memory-full.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

WebAssembly Bulk Memory 提案 (`docs/spec/spec-1.0.0/proposals/bulk-memory-operations/Overview.md`) の
`memory.copy`・`memory.fill`・`memory.init`・`table.copy`・`table.init`・`elem.drop`・`data.drop` を
T3 emitter で活用する。現在は `array.new_data` で passive data segment を消費しているが、
メモリ間コピーやゼロ初期化には bulk memory 命令が高速 (SIMD 等でランタイムが最適化可能)。

## 受け入れ条件

1. `std/bytes` の `memcpy` 相当関数が `memory.copy` を emit する
2. ゼロ初期化バッファが `memory.fill 0` を emit する
3. `table.copy` を使った関数テーブルのコピーサポート
4. 対応する MIR intrinsic を `std/wasm` に追加 (`wasm_memory_copy`, `wasm_memory_fill`)

## 実装タスク

1. `ark-wasm/src/emit/t3_wasm_gc.rs`: `memory.copy` / `memory.fill` emit ヘルパー追加
2. `std/wasm/mod.ark`: `memory_copy(dst, src, len)` / `memory_fill(ptr, val, len)` 追加
3. `std/bytes/mod.ark`: 内部実装で `wasm::memory_copy` を呼ぶように変更

## 参照

- `docs/spec/spec-1.0.0/proposals/bulk-memory-operations/Overview.md`

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
