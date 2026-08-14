# Arukellt Capability Surface

> **Generated** from `docs/data/capabilities.toml` by `scripts/gen/generate-structured-state-docs.py`.
> Do not hand-edit the matrix. Edit the TOML instead.
>
> **Do not treat “registered” or “compiles” as “user-reachable”.**

## Status axes

| Axis | Meaning |
|------|---------|
| `declared` | Named in ADR-011 / design surface |
| `registered` | Present in `std/manifest.toml` |
| `compiles` | Selfhost compile path accepts the module for at least one target |
| `links` | Emitted Wasm links against required host imports |
| `runs` | At least one runtime fixture exercises the module end-to-end |
| `user_reachable` | End users can import and call it on a supported public path |
| `grant_required` | Runtime capability grant / deny flags needed |
| `verified_on` | Targets / host profiles with evidence |

## Module matrix

| Module | Path | declared | registered | compiles | links | runs | user_reachable | grant_required | verified_on | Notes |
|--------|------|:--------:|:----------:|:--------:|:-----:|:----:|:--------------:|:--------------:|-------------|-------|
| `std::host::stdio` | `std/host/stdio.ark` | yes | yes | yes | yes | yes | yes | no | `wasm32`, `wasm32-gc` | Always available |
| `std::host::fs` | `std/host/fs.ark` | yes | yes | yes | yes | yes | yes | yes (--dir) | `wasm32`, `wasm32-gc` | Deny-by-default |
| `std::host::env` | `std/host/env.ark` | yes | yes | yes | yes | yes | yes | no | `wasm32`, `wasm32-gc` |  |
| `std::host::process` | `std/host/process.ark` | yes | yes | yes | yes | yes | yes | optional compile-time deny | `wasm32`, `wasm32-gc` | exit/abort are portable runtime operations; id() returns a stable Err on WASI 0.2. |
| `std::host::clock` | `std/host/clock.ark` | yes | yes | yes | yes | yes | yes | intended deny flags (not in selfhost CLI) | `wasm32`, `wasm32-gc` | Module runs. Intended deny: compile-time MIR on run (#291). Selfhost CLI has no --deny-clock; fixtures in DIAG_PARITY_SKIP (#459). |
| `std::host::random` | `std/host/random.ark` | yes | yes | yes | yes | yes | yes | intended deny flags (not in selfhost CLI) | `wasm32`, `wasm32-gc` | Module runs. Intended deny: compile-time MIR on run (#291). Selfhost CLI has no --deny-random; fixtures in DIAG_PARITY_SKIP (#459). |
| `std::host::http` | `std/host/http.ark` | yes | yes | yes | yes | yes | yes | runtime network/HTTP grant | `wasm32-gc` | Real WASI 0.2 HTTP via the checked P2 runtime adapter (#841); no Arukellt host shim. |
| `std::host::sockets` | `std/host/sockets.ark` | yes | yes | yes | yes | yes | yes | runtime network grant | `wasm32-gc` | Real WASI 0.2 TCP/streams via the checked P2 runtime adapter (#841); wasm32 remains target-gated. |
| `std::host::udp` | `std/host/udp.ark` | yes | yes | partial | partial | no | **no** | n/a | — | Same class as sockets |

## Deny enforcement (structured)

| Module | Flag | Current enforcement | Intended | Transitive | Applies to |
|--------|------|---------------------|----------|:----------:|------------|
| `std::host::process` | `--deny-process` | `compile_time_mir` | `compile_time_mir` | yes | `compile, run, check` |
| `std::host::clock` | `--deny-clock` | `unimplemented` | `compile_time_mir` | yes | `run` |
| `std::host::random` | `--deny-random` | `unimplemented` | `compile_time_mir` | yes | `run` |

## Runtime verification / evidence (not a reachability claim)

1. **`wasm32` / `wasm32-gc` fixtures** — runnable programs under `tests/fixtures/` for modules marked `runs=yes`.
2. **`wasm32-gc` WASM validation** — `scripts/check/check-t3-wasm-validate.py` (historical script name).
3. **Selfhost fixpoint** — compiler uses `stdio` / `fs` under real workloads.
4. **Gate-136** — `scripts/check/gate-136-std-host-rollout.py` checks ADR-011 module presence/docs.

Further user-reachability work: issue #675.

## See also

- [`docs/data/capabilities.toml`](data/capabilities.toml) — SSOT
- [`docs/current-state.md`](current-state.md)
- [`docs/platform/target-runtime-and-surfaces.md`](platform/target-runtime-and-surfaces.md)
