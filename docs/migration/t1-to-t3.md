# Migration Guide: T1 → T3

This guide helps users migrate from `wasm32-wasi-p1` (T1) to `wasm32-wasi-p2` (T3).

## Overview

| Aspect | T1 (wasm32-wasi-p1) | T3 (wasm32-wasi-p2) |
|--------|---------------------|---------------------|
| Memory model | Linear memory (bump allocator) | Wasm GC (struct/array) |
| WASI version | Preview 1 | Preview 2 |
| Component Model | Not supported | Supported |
| WIT generation | Not supported | Auto-generated |
| Default emit | `core-wasm` | `core-wasm` (will become `component`) |
| Status | Stable, maintained | Implemented (T1 fallback) |

## CLI Changes

### Explicit target selection

```bash
# T1 (current default)
arukellt run file.ark
arukellt run --target wasm32-wasi-p1 file.ark

# T3
arukellt run --target wasm32-wasi-p2 file.ark
```

### Deprecated aliases

The following target names are deprecated and will emit warnings:

| Old name | New canonical name |
|----------|-------------------|
| `wasm32-wasi` | `wasm32-wasi-p1` |
| `wasm-gc` | `wasm32-wasi-p2` |
| `wasm-gc-wasi-p2` | `wasm32-wasi-p2` |
| `wasm32` | `wasm32-freestanding` |

### Emit kinds

```bash
# T1: only core-wasm supported
arukellt compile --target wasm32-wasi-p1 --emit core-wasm file.ark

# T3: core-wasm, component, wit, all
arukellt compile --target wasm32-wasi-p2 --emit component file.ark
arukellt compile --target wasm32-wasi-p2 --emit wit file.ark
```

## Capability Flags

T3 introduces capability-based runtime access:

```bash
# Default: inherits stdio + preopens current directory (backward compatible)
arukellt run file.ark

# Explicit directory grants
arukellt run --dir /data:ro --dir /output:rw file.ark

# Deny filesystem access entirely
arukellt run --deny-fs file.ark

# Deny clock/random
arukellt run --deny-clock --deny-random file.ark
```

## Code Compatibility

All v0 and v1 Arukellt source code is compatible with both T1 and T3. The difference is in code generation and runtime behavior, not language features.

## Future Changes

- Default target will eventually switch from T1 to T3
- T3 will emit `component` by default (currently `core-wasm`)
- T1 will remain supported as a maintained backend for environments requiring linear memory (e.g., AtCoder)

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `unsupported emit kind 'component' for target wasm32-wasi-p1` | T1 doesn't support component model | Use `--target wasm32-wasi-p2` or `--emit core-wasm` |
| `target 'wasm-gc' is deprecated` | Using old target alias | Switch to `--target wasm32-wasi-p2` |
| `target 'wasm32-freestanding' is not yet implemented` | T2 is planned, not implemented | Use T1 or T3 |
