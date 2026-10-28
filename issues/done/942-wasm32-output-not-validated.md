---
Status: done
Created: 2026-10-27
Updated: 2026-10-28
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

- [x] `compile` / `build` / `run` for `wasm32` validate the emitted module
      (the same validator path as `wasm32-gc`) and fail on invalid output.
- [x] A regression test feeds a known-invalid emission (e.g. the #940 repro
      while it is still broken, or an injected fault) and asserts a non-zero
      exit and no success message.
- [x] `docs/current-state.md` W0004 statement is true for every target.

## Progress (2026-10-27)

- `scripts/run/arukellt-selfhost.sh`: `compile` / `build` with `-o` run the
  compiler with captured output, validate the emitted core module with the
  same `$WASM_TOOLS_BIN validate` used by the component path, and on failure
  drop the artifact, suppress `compilation succeeded`, and exit 1 with
  `W0004 generated Wasm failed validation`. `run` validates the module it is
  about to execute on the same path.
- `scripts/check/check-cli-guarantees.py compile-validates-output` compiles
  the #940 same-name-type repro and asserts rejection (W0004, no artifact, no
  success line); once #940 lands and the module is valid, the same check
  requires the artifact to validate, so it does not pin the test to the bug.
  It also asserts a valid program still compiles. Wired into
  `check-cli-guarantees.py all`, which `verify quick` runs.
- Verified: `compile` of the #940 repro exits 1 with W0004 and leaves no file;
  `compile`/`run` of a valid program and `--target wasm32-gc` core compile
  still succeed; `check-cli-guarantees.py all` passes; `verify lane` passes.

## Re-verification — 2026-10-28 (close evidence)

Acceptance items were re-checked on this checkout (default pinned artifact,
`PATH` with wasmtime 46.0.1):

- `python3 scripts/check/check-cli-guarantees.py compile-validates-output` →
  exit 0 (rejects the #940 same-name-type repro with W0004, no success line,
  no artifact left; the valid program compiles and validates). The check is
  wired into `check-cli-guarantees.py all` (`scripts/manager.py` quick pass).
- Manual `compile --target wasm32` of `.build/cli-guarantees/samename/main.ark`
  → exit 1, `error — W0004 generated Wasm failed validation`, no `-o` file.
- Manual `compile --target wasm32-gc` of the same repro → exit 1, W0004
  (`type mismatch: expected anyref, found f64`), no artifact.
- Manual `run --target wasm32` of the same repro → exit 1, W0004 (no
  `compilation succeeded` reported as success, nothing executed).
- Valid program (`compile --target wasm32` and `--target wasm32-gc`) → exit 0,
  artifacts pass `wasm-tools validate`.
- `docs/current-state.md`: "Wasm validation is a hard error (W0004)" and the
  generated diagnostics table (`W0004`, error, `backend-validate`,
  implemented, emitted) are true for both Wasm targets; `native-cpp` /
  `native-llvm` are scaffolds that do not emit Wasm, so W0004 does not apply
  there. `docs/compiler/diagnostics.md` states "Any W0004 failure stops the
  build" and has no current warning-only caveat (warning-only appears only
  under the old-document cautions).

The #940 repro is still invalid (the guarantee check exercises the rejection
branch), so #940 stays open; this issue's contract (reject + no artifact + no
success message) is satisfied. Close candidate.

