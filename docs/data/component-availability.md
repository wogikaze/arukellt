# Component availability (structured)

> **Generated** from `docs/data/component-availability.toml`.
> Do not flatten to a single `available: true/false`.

- Target: `wasm32-gc`
- Public contract: ADR-054 component/wit core output with official WASI P2 packaging
- Implementation: The selfhost compiler emits the standard WASI P2 core ABI and the launcher uses the official wasm-tools component pipeline. Repository-specific bridges, post-linkers, and legacy compatibility paths are not supported.

## Active compiler artifacts

| Role | Path |
|------|------|
| Pinned bootstrap | `bootstrap/arukellt-selfhost.wasm` |
| Recommended for library exports | `.build/selfhost/arukellt-s2.wasm` |
| Env override | `ARUKELLT_SELFHOST_WASM` |

## Surfaces

| ID | Label | Status | Active compiler | External tools | Notes |
|----|-------|--------|-----------------|----------------|-------|
| `command_component` | Command component (wasi:cli/run) | `available_on_pinned` | `pinned_or_s2` | required (wasm-tools component embed/new) | The compiler emits standard P2 core imports; the launcher packages them with the official WASI CLI WIT world. gate-510 validation + gate-714 direct Wasmtime execution cover the command path. |
| `library_component` | Library component (pub fn exports) | `requires_s2` | `s2_or_ARUKELLT_SELFHOST_WASM` | required (wasm-tools component embed/new) | Library-style pub fn exports use the compiler's WIT output and official component tooling; no Rust bindings or repository-specific adapter is required. |
| `wit_emit` | WIT emit | `partial` | `s2_preferred` | optional | Scalar library WIT is emitted by the selfhost compiler; the official component toolchain consumes it for packaging. |
| `jco_transpile` | jco transpile + Node.js execution | `smoke` | `s2_or_ARUKELLT_SELFHOST_WASM` | jco >= 1.25.2; Node.js >= 18 | Scalar pub fn exports execute via jco-generated JS glue. Node 23.6 requires --experimental-wasm-memory64 due to T3 i64 memory. String/record/variant canonical ABI adapters still pending (#029 area). |
| `component_cli` | arukellt component build | `provisional` | `same_as_compile` | required (wasm-tools for packaging/inspect/validate) | Aliases compile --emit component and use official wasm-tools for packaging; inspect/validate remain delegated to wasm-tools. |
