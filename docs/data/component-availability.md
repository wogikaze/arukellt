# Component availability (structured)

> **Generated** from `docs/data/component-availability.toml`.
> Do not flatten to a single `available: true/false`.

- Target: `wasm32-gc`
- Public contract: ADR-008 in-tree component/wit emit
- Implementation: P2 command path is bridged in-tree (#714). Library/other packaging may still use wasm-tools helpers.

## Active compiler artifacts

| Role | Path |
|------|------|
| Pinned bootstrap | `bootstrap/arukellt-selfhost.wasm` |
| Recommended for library exports | `.build/selfhost/arukellt-s2.wasm` |
| Env override | `ARUKELLT_SELFHOST_WASM` |

## Surfaces

| ID | Label | Status | Active compiler | External tools | Notes |
|----|-------|--------|-----------------|----------------|-------|
| `command_component` | Command component (wasi:cli/run) | `available_on_pinned` | `pinned_or_s2` | optional (wasm-tools for inspect) | P2 command uses bridged in-tree emit (#714). gate_510 validate + gate-714 wasmtime run (hello p2) green. Guest-native get-stdout lowering remains #668. |
| `library_component` | Library component (pub fn exports) | `requires_s2` | `s2_or_ARUKELLT_SELFHOST_WASM` | optional | Pinned bootstrap returns empty WIT / non-invokable components for library-style pub fn exports (#666) |
| `wit_emit` | WIT emit | `partial` | `s2_preferred` | optional | Scalar library wit works on s2; pinned may emit empty library WIT |
| `jco_transpile` | jco transpile + Node.js execution | `smoke` | `s2_or_ARUKELLT_SELFHOST_WASM` | jco >= 1.25.2; Node.js >= 18 | Scalar pub fn exports execute via jco-generated JS glue. Node 23.6 requires --experimental-wasm-memory64 due to T3 i64 memory. String/record/variant canonical ABI adapters still pending (#029 area). |
| `component_cli` | arukellt component build | `provisional` | `same_as_compile` | none for build | Aliases compile --emit component; inspect/validate not implemented |
