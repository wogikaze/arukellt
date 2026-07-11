# Component availability (structured)

> **Generated** from `docs/data/component-availability.toml`.
> Do not flatten to a single `available: true/false`.

- Target: `wasm32-gc`
- Public contract: ADR-008 in-tree component/wit emit
- Implementation: Living path may still use wasm-tools / Python wrap helpers for some packaging steps

## Active compiler artifacts

| Role | Path |
|------|------|
| Pinned bootstrap | `bootstrap/arukellt-selfhost.wasm` |
| Recommended for library exports | `.build/selfhost/arukellt-s2.wasm` |
| Env override | `ARUKELLT_SELFHOST_WASM` |

## Surfaces

| ID | Label | Status | Active compiler | External tools | Notes |
|----|-------|--------|-----------------|----------------|-------|
| `command_component` | Command component (wasi:cli/run) | `available_on_pinned` | `pinned_or_s2` | optional (wasm-tools for inspect) | Modules without exportable pub fn use P2 command wrapper |
| `library_component` | Library component (pub fn exports) | `requires_s2` | `s2_or_ARUKELLT_SELFHOST_WASM` | optional | Pinned bootstrap returns empty WIT / non-invokable components for library-style pub fn exports (#666) |
| `wit_emit` | WIT emit | `partial` | `s2_preferred` | optional | Scalar library wit works on s2; pinned may emit empty library WIT |
| `component_cli` | arukellt component build | `provisional` | `same_as_compile` | none for build | Aliases compile --emit component; inspect/validate not implemented |
