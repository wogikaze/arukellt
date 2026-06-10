---
Status: done
Created: 2026-04-03
Updated: 2026-06-10
ID: 475
Track: cli
---

# arukellt component subcommand

## Summary

The `arukellt component` subcommand was implemented in the Ark selfhost
compiler (`src/compiler/main.ark`) with three sub-subcommands:

- `component build <file.ark>` — compile to component WASM (delegates to
  compile pipeline with `--emit component --target wasm32-wasi-p2`)
- `component inspect <file.wasm>` — prints a helpful message referencing
  `wasm-tools component wit` (not yet implementable from WASI sandbox)
- `component validate <file.wasm>` — prints a helpful message referencing
  `wasm-tools validate` (same constraint)

## Resolution

All acceptance items are checked. The core `build` subcommand is fully
functional. `inspect` and `validate` have graceful stubs that direct users
to the appropriate `wasm-tools` commands, since process spawning is
unavailable from within the WASI sandbox.

## Acceptance

- [x] `arukellt component --help` shows subcommands
- [x] `arukellt component build <file.ark>` compiles to component.wasm
- [x] `arukellt component inspect <file.wasm>` prints helpful wasm-tools hint
- [x] `arukellt component validate <file.wasm>` prints helpful wasm-tools hint
- [x] `python scripts/manager.py verify` passes

## Evidence

- CLI parity tests 15-17 cover all three sub-subcommands
- Verification passes 23/23
