# Wasm 機能レイヤー

> **Current-first**: 実装の現在地は [`../current-state.md`](../current-state.md) を参照。
> このページは current deployment reality と、そこから見た target layering を整理する。

## 現在の reality

- production path は **T1 `wasm32-wasi-p1`**
- T3 `wasm32-wasi-p2` は experimental fallback
- T3 `wasm32-wasi-p2` は **experimental fallback**
- T3 も現在の run path では P1 runtime / linear-memory-oriented 実装に依存する
- `--emit component` is a hard error
- `--emit all` is also blocked for the same reason
- backend validation failure (`W0004`) is a hard error

## Current target view

| Target | Tier | Current status | Notes |
|-------|------|----------------|-------|
| `wasm32-wasi-p1` | T1 | Implemented | default production path |
| `wasm32-freestanding` | T2 | Not implemented | registry only |
| `wasm32-wasi-p2` | T3 | Experimental | fallback to current T1-oriented runtime path |
| `native` | T4 | Not implemented | LLVM scaffold only |
| `wasm32-wasi-p3` | T5 | Future | not started |

## Alias policy

旧ターゲット alias は受理されるが `W0002` を出す。
canonical 名を使うこと。

- `wasm32-wasi` → `wasm32-wasi-p1`
- `wasm-gc` → `wasm32-wasi-p2`
- `wasm-gc-wasi-p2` → `wasm32-wasi-p2`
- `wasm32` → `wasm32-freestanding`

## Current layering guidance

### Layer A: shipping reality

- core Wasm artifact
- WASI Preview 1 run path
- linear-memory-oriented backend/runtime assumptions

### Layer B: experimental compatibility surface

- T3 target name exists
- T3 run path works for selected cases
- current implementation is still fallback-oriented, not full Preview 2 / Component Model delivery

### Layer C: future design space

- true Wasm GC lowering
- Component Model deployment
- native backend completion
- p3/async-first runtime work

## What not to assume from old docs

古い文書や ADR を読んでも、以下を current reality と誤読しないこと。

- T3 が full Component Model deploy path である
- WIT/component emit が使える
- backend validation が warning-only のままである
- T1 details が消えている

## Runtime model terminology

The `BackendPlan` uses `RuntimeModel` to distinguish executable reality from target intent:

| RuntimeModel | Meaning | Current state |
|-------------|---------|---------------|
| `T1LinearP1` | Linear memory + WASI P1 | Active, production |
| `T3FallbackToT1` | T3 target, but still using T1 linear memory internally | Transitional (current T3) |
| `T3WasmGcP2` | True WasmGC-native data model + WASI P2 | Target for v1 exit |
| `T4LlvmScaffold` | LLVM native scaffold | Optional, not v1 gate |

## T3 String representation (bridge mode)

T3 uses a **linear-memory bridge** for String values:

- **Layout**: `[len:4 bytes LE][data bytes]` — pointer (i32) points to data start; length at `ptr - 4`
- **GC type section**: A `(type $string (struct (field (ref (array i8))))` is declared but **not used at runtime**
- **Allocation**: Bump allocator (global 0); no GC collection yet

### Implemented operations

| Operation | Status | Notes |
|-----------|--------|-------|
| `String_from` | ✓ | Identity (passthrough) |
| `concat` | ✓ | Allocates new string, copies both halves |
| `to_string` | ✓ | Polymorphic: i32→helper fn, f64/i64/bool→stubs |
| `string_len` | ✓ | Reads `[ptr-4]` |
| `char_at` | ✓ | Loads single byte at offset |
| `substring` | ✓ | Allocates new string from range |
| `clone` | ✓ | Full copy via memory.copy |
| `to_uppercase` / `to_lowercase` | ✓ | ASCII-only, clone + in-place transform |
| `trim` | ✓ | Scans whitespace from both ends, returns substring |
| `contains` | ✓ | Byte-by-byte substring search |
| `starts_with` / `ends_with` | ✓ | Prefix/suffix byte comparison |
| `replace` | stub | Returns clone (ignores replacement args) |
| `split` | stub | Returns empty vec |
| `i32_to_string` | ✓ | Helper function with div/mod loop |
| `f64_to_string` | stub | Pushes 0 |
| `i64_to_string` | stub | Falls through to default |

### Design rationale

Bridge mode keeps the T3 emitter operational without requiring full WasmGC runtime support
(reference counting, GC arrays). When WasmGC runtimes mature, String can migrate to
`(array i8)` with minimal API change since the byte-oriented layout is preserved.

- [../current-state.md](../current-state.md)
- [abi.md](abi.md)
- [../process/policy.md](../process/policy.md)
- [../migration/t1-to-t3.md](../migration/t1-to-t3.md)
