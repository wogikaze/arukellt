---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 942
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "W0004, #940"
---

# 942 — `wasm32` compile reports success for invalid core Wasm

## Summary

`docs/current-state.md` states "Wasm validation is a hard error (W0004)".
On `wasm32-gc` an invalid module is rejected ("compiler produced invalid
core Wasm"). On `wasm32` the same class of bug is written to disk and
reported as "compilation succeeded", and the failure only appears when the
module is run.

## Evidence (2026-10-27)

- The #940 repro (`samename/main.ark`) with
  `.build/selfhost/arukellt-s2-runtime.wasm --target wasm32` prints
  `compilation succeeded (phase 6)` / `wrote 4226 bytes`; then
  `wasm-tools validate` fails with `func 11 failed to validate`, and
  `wasmtime run` fails with `type mismatch: expected i32, found f64`.

## Expected

Every target's output passes core validation before the CLI reports success;
an invalid module is an internal compiler error with exit code ≠ 0 and no
artifact left as if it were valid.

## Acceptance

- [ ] `compile` / `build` / `run` for `wasm32` validate the emitted module
      (the same validator path as `wasm32-gc`) and fail on invalid output.
- [ ] A regression test feeds a known-invalid emission (e.g. the #940 repro
      while it is still broken, or an injected fault) and asserts a non-zero
      exit and no success message.
- [ ] `docs/current-state.md` W0004 statement is true for every target.
