# Operational Policy

## Capability Defaults

| Capability | Default | Override |
|------------|---------|----------|
| Standard I/O (stdin/stdout/stderr) | Allow | Cannot be denied |
| Filesystem access | Allow (current dir, rw) | `--deny-fs`, `--dir path:ro` |
| Clock/time | Allow | `--deny-clock` |
| Random numbers | Allow | `--deny-random` |

> **Future change**: Filesystem default will switch to deny. Users will need `--dir .` to enable.

## Generated Artifacts

| Artifact | Commit? | Regeneration |
|----------|---------|--------------|
| WIT files (`*.wit`) | Yes | `mise run regen-wit` |
| Target reference | Yes | Auto-generated from `ark-target` |
| CLI help text | No | Built from source |

## Tool Version Pinning

All external tools are version-pinned in `mise.toml`:

| Tool | Purpose | Pin method |
|------|---------|------------|
| Rust | Compiler toolchain | `rust-toolchain.toml` or `mise.toml` |
| wasmtime | Wasm runtime | `Cargo.toml` dependency |
| wasm-tools | Component wrapping | `mise.toml` |

## Target Support Policy

| Target | Status | Support level |
|--------|--------|---------------|
| `wasm32-wasi-p1` (T1) | Implemented | Full CI, backward-compatible |
| `wasm32-freestanding` (T2) | Planned | Registry only |
| `wasm32-wasi-p2` (T3) | Implemented | Full CI, T1 fallback |
| `native` (T4) | Planned | Registry only |
| `wasm32-wasi-p3` (T5) | Future | Registry only |

## Security Boundaries

1. **No GC reference export**: GC-managed references cannot be directly exported to C/host
2. **No C pointer import**: Raw C pointers cannot be converted to GC references
3. **Capability isolation**: Runtime grants are explicit, not inherited
4. **WIT type safety**: Only WIT-exportable types can appear in component exports

## Diagnostic Quality Standards

- Every error includes: diagnostic code, expected vs actual, source location
- Fix-it hints for common mistakes
- Target-specific errors include target name and tier
- LLM-friendly format: structured, deterministic, no ambiguous phrasing
