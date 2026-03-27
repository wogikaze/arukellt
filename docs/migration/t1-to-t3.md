# Migration Guide: T1 → T3

> **Current-first**: this page explains migration expectations against the current implementation, not an aspirational full T3 rollout.

## Overview

| Aspect | T1 (wasm32-wasi-p1) | T3 (wasm32-wasi-p2) |
|--------|---------------------|---------------------|
| Memory model in current implementation | Linear memory (bump-oriented runtime path) | Experimental fallback to the same runtime path |
| WASI version in shipped run path | Preview 1 | Preview 1 internally today |
| Component Model | Hard error | Hard error |
| WIT generation | Limited/partial tooling only | Design/migration context, not current deployment contract |
| Default emit | `core-wasm` | `core-wasm` |
| Status | Stable default | Experimental fallback |

## CLI Selection

```bash
# T1 (current default)
arukellt run file.ark
arukellt run --target wasm32-wasi-p1 file.ark

# T3 experimental fallback
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
# Current supported path
arukellt compile --target wasm32-wasi-p1 --emit core-wasm file.ark
arukellt compile --target wasm32-wasi-p2 --emit core-wasm file.ark
```

Current behavior:

- `--emit component` → hard error
- `--emit all` → hard error
- T3 does not currently provide a production component-model deployment surface

## Runtime / Capability Notes

Current runtime surface is stricter than some older docs imply.

- No `--dir` flag = no filesystem access
- `--deny-fs` is supported
- `--deny-clock` is a hard error (not enforced capability filtering yet)
- `--deny-random` is a hard error (not enforced capability filtering yet)

## V1 Core Exit

The v1 core exit gate is **T3 core-wasm compile/run completion** — not Component Model completion. Specifically:

- T3 must compile and run all fixture categories using WasmGC-native data representations.
- `RuntimeModel::T3FallbackToT1` must be replaced by a truthful non-fallback runtime model.
- `--emit component` remains out of scope for v1 and continues to be a hard error.
- T1 (`wasm32-wasi-p1`) is retained as a compatibility path for non-GC environments only.

See `docs/current-state.md` § V1 Exit Criteria for the canonical definition.

## Code Compatibility

Language-level source compatibility remains the goal. The current difference is runtime/backend path, not frontend syntax.

## What changed in this refactor track

The intentional behavior change owned by this migration/update work is:

- `W0004` (generated Wasm failed validation) is now treated as a build failure

This is a quality-gate change, not a source-language change.

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `invalid emit kind` / component hard error | Component output is not implemented | Use `--emit core-wasm` |
| `target alias ... is deprecated` | Old alias accepted with `W0002` | Switch to canonical target name |
| `target ... is not yet implemented` | T2/T4/T5 are not current run paths | Use T1 or experimental T3 |
| `generated Wasm module failed validation` | Backend validation failed (`W0004`) | Treat as compiler/backend failure, not a warning to ignore |
