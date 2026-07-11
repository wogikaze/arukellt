# Arukellt Capability Surface

> Host capability surface for `std::host::*`.  
> **Do not treat “registered” or “compiles” as “user-reachable”.**  
> Axes below follow the docs audit 2026-07-11 multi-axis contract.
> Structured flags also live in `docs/data/project-state.toml` (`[contract_audit]`).

## Status axes

| Axis | Meaning |
|------|---------|
| `declared` | Named in ADR-011 / design surface |
| `registered` | Present in `std/manifest.toml` |
| `compiles` | Selfhost compile path accepts the module for at least one target |
| `links` | Emitted Wasm links against required host imports |
| `runs` | At least one runtime fixture exercises the module end-to-end |
| `user_reachable` | End users can import and call it on a supported public path without internal-only bridges |
| `grant_required` | Runtime capability grant (`--dir`, deny flags, etc.) needed |
| `verified_on` | Targets / host profiles with evidence |

## Module matrix

| Module | Path | declared | registered | compiles | links | runs | user_reachable | grant_required | verified_on | Notes |
|--------|------|:--------:|:----------:|:--------:|:-----:|:----:|:--------------:|:--------------:|-------------|-------|
| `std::host::stdio` | `std/host/stdio.ark` | yes | yes | yes | yes | yes | yes | no | `wasm32`, `wasm32-gc` | Always available |
| `std::host::fs` | `std/host/fs.ark` | yes | yes | yes | yes | yes | yes | yes (`--dir`) | `wasm32`, `wasm32-gc` | Deny-by-default |
| `std::host::env` | `std/host/env.ark` | yes | yes | yes | yes | yes | yes | no | `wasm32`, `wasm32-gc` | |
| `std::host::process` | `std/host/process.ark` | yes | yes | yes | yes | yes | yes | no | `wasm32`, `wasm32-gc` | |
| `std::host::clock` | `std/host/clock.ark` | yes | yes | yes | yes | yes | yes | deny flags | `wasm32`, `wasm32-gc` | `--deny-clock` hard error |
| `std::host::random` | `std/host/random.ark` | yes | yes | yes | yes | yes | yes | deny flags | `wasm32`, `wasm32-gc` | `--deny-random` hard error |
| `std::host::http` | `std/host/http.ark` | yes | yes | partial | partial | no | **no** | n/a | — | `host_http_user_reachable=false` |
| `std::host::sockets` | `std/host/sockets.ark` | yes | yes | partial | partial | no | **no** | n/a | — | E0500 on `wasm32`; not user-reachable |
| `std::host::udp` | `std/host/udp.ark` | yes | yes | partial | partial | no | **no** | n/a | — | Same class as sockets |

## Runtime verification / evidence (not a reachability claim)

1. **`wasm32` / `wasm32-gc` fixtures** — runnable programs under `tests/fixtures/` for modules marked `runs=yes`.
2. **`wasm32-gc` WASM validation** — `scripts/check/check-t3-wasm-validate.py` (historical script name).
3. **Selfhost fixpoint** — compiler uses `stdio` / `fs` under real workloads.
4. **Gate-136** — `scripts/check/gate-136-std-host-rollout.py` checks ADR-011 module presence/docs.

Further user-reachability and permission-flag work: issue #675.

## See also

- [`docs/current-state.md`](current-state.md) — Known Limitations
- [`docs/platform/target-runtime-and-surfaces.md`](platform/target-runtime-and-surfaces.md)
- [`docs/data/project-state.toml`](data/project-state.toml)
