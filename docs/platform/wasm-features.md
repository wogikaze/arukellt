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

## 関連

- [../current-state.md](../current-state.md)
- [abi.md](abi.md)
- [../process/policy.md](../process/policy.md)
- [../migration/t1-to-t3.md](../migration/t1-to-t3.md)
