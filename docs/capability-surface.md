# Arukellt Capability Surface

> This document enumerates the host capability surface exposed through
> `std::host::*` modules and the runtime verification status for each.

## std::host modules

The following modules implement the ADR-011 host capability surface:

| Module | Path | Description |
|--------|------|-------------|
| `std::host::stdio` | `std/host/stdio.ark` | Standard input/output streams |
| `std::host::fs` | `std/host/fs.ark` | File system read/write operations |
| `std::host::env` | `std/host/env.ark` | Environment variable access |
| `std::host::process` | `std/host/process.ark` | Process exit and argument access |
| `std::host::clock` | `std/host/clock.ark` | Wall clock and monotonic time |
| `std::host::random` | `std/host/random.ark` | Random number generation |
| `std::host::http` | `std/host/http.ark` | HTTP client operations (not user-reachable) |
| `std::host::sockets` | `std/host/sockets.ark` | TCP socket operations |

All modules are registered in `std/manifest.toml` and compiled via the
selfhost compiler to `wasm32`（WASI P1 host profile）and `wasm32-gc`
（WASI P2 host profile）targets. Legacy CLI aliases `wasm32-wasi-p1` /
`wasm32-wasi-p2` may still appear in fixtures and tooling.

## Runtime verification

Each `std::host::*` module is verified at runtime through:

1. **`wasm32` fixtures** — `tests/fixtures/` contain runnable programs that
   exercise each host capability end-to-end via `wasmtime run`.
2. **`wasm32-gc` WASM validation** — `scripts/check/check-t3-wasm-validate.py`
   （historical script name）compiles every fixture and validates the emitted WASM against
   `wasm-tools validate`.
3. **Selfhost fixpoint** — the selfhost compiler itself uses
   `std::host::stdio` and `std::host::fs` for file I/O, ensuring
   these capabilities work under real workloads.
4. **Gate-136 enforcement** — this document is checked by
   `scripts/check/gate-136-std-host-rollout.py` to ensure all
   ADR-011 modules are present and documented.
