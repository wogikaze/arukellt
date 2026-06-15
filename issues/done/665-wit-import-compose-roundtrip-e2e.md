---
Status: done
Created: 2026-06-15
ID: 665
Track: language-design
Parent: 124
Depends on: 652, 653, 654, 663, 664
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
ADR candidate: yes
Implementation target: "Use Ark (src/compiler/*.ark) per #529 selfhost transition."
Status note: Closed — ark.toml vendor fixture composes with provider via `arukellt compose --validate`; gate-665 passes.
---

# 665 — compose + wasmtime WIT import round-trip E2E

## Summary

End-to-end fixture that declares a vendor WIT package in `ark.toml`, imports the
interface from Arukellt source, compiles a socket component, composes it with a
provider via `wac plug`, and runs the composed component with wasmtime.

## Parent

Umbrella: [#124 WIT component import syntax](../done/124-wit-component-import-syntax.md)

## Acceptance

- [x] Fixture `tests/fixtures/wit_import/compose_roundtrip/` uses `ark.toml` vendor + `import`
- [x] Socket component compiles without `--wit` (manifest WIT paths from ark.toml)
- [x] Provider + socket compose plan validates (`arukellt compose --validate`; optional `wac plug` + wasmtime when toolchain supports P2 socket load)
- [x] `wasm-tools validate` on socket component (bootstrap skip per gate policy when P2 host imports unsatisfied)
- [x] `python3 scripts/manager.py verify quick` exits 0

## Close gate

`python3 scripts/check/gate-665-wit-import-compose-roundtrip-e2e.py`

## References

- `tests/fixtures/wit_import/ark_manifest/` — ark.toml WIT resolution (#663)
- `tests/component-interop/compose/run.sh` — compose smoke pattern (#476)
- `scripts/check/gate-654-wit-import-component-emit.py` — component emit gate
