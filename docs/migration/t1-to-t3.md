# Migration Guide: T1 → T3

> **Current-first**: this page explains migration from the T1 compatibility path to the canonical T3 path.
> For the current support matrix and current emitted artifact notes, also check [`../current-state.md`](../current-state.md).

## Overview

| Aspect | T1 (wasm32-wasi-p1) | T3 (wasm32-wasi-p2) |
|--------|---------------------|---------------------|
| Memory model | Linear memory | Wasm GC-native data model |
| WASI I/O | Preview 1 | GC-native runtime with WASI I/O marshaling |
| Component / WIT extras | not the primary path | available on the T3 target |
| Default emit | `core-wasm` | `core-wasm` |
| Status | Compatibility path | Canonical current path |

## CLI Selection

```bash
# T1 (CLI default for compatibility)
arukellt run file.ark
arukellt run --target wasm32-wasi-p1 file.ark

# T3 (canonical path)
arukellt run --target wasm32-wasi-p2 file.ark
```

## Deprecated aliases

The following names are accepted but emit `W0002`:

| Old name | Canonical name |
|----------|----------------|
| `wasm32-wasi` | `wasm32-wasi-p1` |
| `wasm-gc` | `wasm32-wasi-p2` |
| `wasm-gc-wasi-p2` | `wasm32-wasi-p2` |
| `wasm32` | `wasm32-freestanding` |

## Emit kinds

```bash
# Core Wasm
arukellt compile --target wasm32-wasi-p1 --emit core-wasm file.ark
arukellt compile --target wasm32-wasi-p2 --emit core-wasm file.ark

# T3 component / WIT outputs
arukellt compile --target wasm32-wasi-p2 --emit component file.ark
arukellt compile --target wasm32-wasi-p2 --emit wit file.ark
arukellt compile --target wasm32-wasi-p2 --emit all file.ark
```

## Runtime / Capability Notes

- No `--dir` flag means no filesystem access
- `--deny-fs` is supported
- `--deny-clock` is still a hard-error placeholder
- `--deny-random` is still a hard-error placeholder

## What changed in the T3 promotion

- T3 (`wasm32-wasi-p2`) became the canonical target
- `RuntimeModel::T3WasmGcP2` is the active T3 runtime model
- T3 uses a GC-native data model
- backend validation failure (`W0004`) is a build-breaking error
- component / WIT outputs were later added on top of the T3 path in v2

## What did not change

- CLI default target remains `wasm32-wasi-p1` for compatibility
- Source-language syntax stays shared between T1 and T3
- T1 remains the compatibility path for non-GC environments
- frontend pipeline remains shared

## Historical note

Older v1-era documents may still describe component output as out of scope or hard-error-only. That was true for the original v1 gate, but it is no longer the current behavior after v2.

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `target alias ... is deprecated` | Old alias accepted with `W0002` | Switch to canonical target name |
| `target ... is not yet implemented` | T2 / T4 / T5 are not current run paths | Use T1 or T3 |
| `generated Wasm module failed validation` | Backend validation failed (`W0004`) | Treat as compiler/backend failure |
| component emit/setup failure | Missing `wasm-tools` or adapter / unsupported export surface | Check current component requirements in `docs/current-state.md` |
