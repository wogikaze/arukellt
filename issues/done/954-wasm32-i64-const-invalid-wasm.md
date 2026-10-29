---
Status: done
Created: 2026-10-28
Updated: 2026-10-30
ID: 954
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "ABC481 arukellt port + WAT emission session (2026-10-28, HEAD 0a64bdb62)"
Related: "#933, #935, #936, ADR-053"
---

# 954 — wasm32: referencing a top-level `i64` const produces invalid Wasm

## Summary

A top-level `const NAME: i64 = <literal>` that is referenced from code makes the
`wasm32` backend emit an invalid module:

```
error — W0004 generated Wasm failed validation:
  error: func 10 failed to validate
      0: type mismatch: expected i32, found i64
```

`wasm32-gc` compiles the same program fine, so the fault is in the linear-memory
lowering of constant references.

## Repro

`tmp/i64const.ark`:

```ark
const LIMIT: i64 = 4096i64

fn main() {
    let x: i64 = 100i64 * LIMIT
    if x > 0i64 {
        let y: i32 = 1
        let z: i32 = y + 1
        if z > 1 {
            return
        }
    }
}
```

```
$ export PATH=<wasmtime>:$PATH
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wasm -o .build/i64const.wasm tmp/i64const.ark
arukellt-selfhost: error — W0004 generated Wasm failed validation: .build/i64const.wasm
  error: func 10 failed to validate
      0: type mismatch: expected i32, found i64
```

`--target wasm32-gc --emit wasm` succeeds.

## Workaround

Inline the literal at every use site (this is what ABC481 E and G do). That removes
the const function call from the lowered module.

## Notes

- `i32` consts and local `let` with i64 work; only the top-level const reference fails.
- Same family as #936 (prelude/core symbols dispatched by name with wrong types) and
  #933 (MIR re-infers types by name).

## Acceptance

- [ ] Repro compiles and validates on `wasm32`.
- [ ] Fixture covers i64/i32/f64 consts referenced from expressions and calls on both targets.

## Implementation (2026-10-30)

- `src/compiler/mir/lower/core_names.ark`: a const reference is a value, not a
  function value. `lower_core_name_value` lowers a const-named ident/path
  through `lower_core_const_reference`, which allocates the destination local
  with the const decl's declared return value type (`ctx_fn_return_vt`) instead
  of the CoreHIR expression `val_type`. The frontend has no type for a const
  ident, so the old path made an `i32` local and the binary lowering inserted
  `i64.extend_i32_s` on the i64 call result (`expected i32, found i64`).
- `src/compiler/mir/lower/entry_decls.ark`: const decl bodies are kept through
  the stdlib AST-edge prune, so a const reference's call target always exists.
- Fixture: `tests/fixtures/stdlib_core/const_reference_lowering.ark`
  (file-scope i32/i64/f64 consts in expressions and call arguments, plus a
  multi-const f64 import).
- s2 rebuilt once: `python3 scripts/manager.py selfhost build-compiler` (~55 s).

## Verification (2026-10-30)

- Repro (`const LIMIT: i64 = 4096i64` referenced from `100i64 * LIMIT`):
  `compile --target wasm32 --emit wasm` passes validation and `wasmtime run`
  exits 0.
- Same program with `--target wasm32-gc --emit component` runs under
  `wasmtime run --wasm gc --wasm function-references`, exit 0.
- Fixture `tests/fixtures/stdlib_core/const_reference_lowering.ark` passes on
  both `wasm32` (core + wasmtime) and `wasm32-gc` (component + wasmtime).
- manifest line: `run:stdlib_core/const_reference_lowering.ark` (the Lead
  registers it; `tests/fixtures/manifest.txt` was not edited in this lane).

- [x] Repro compiles and validates on `wasm32`.
- [x] Fixture covers i64/i32/f64 consts referenced from expressions and calls on both targets.

## Close review (2026-10-30, Lead)

Both acceptance items are backed by the tree, not just a probe:

- the repro compiles, validates and runs on `wasm32` (and on `wasm32-gc` as a
  component);
- the fixture `tests/fixtures/stdlib_core/const_reference_lowering.ark` is
  registered in `manifest.txt` and passes in the full harness run.

Evidence for the merged tree (stage-2 sha256 `d8f32674…`):

```
selfhost build-compiler   -> PASS
selfhost fixture-parity   -> PASS=1920 FAIL=0 (COMPILE_VALIDATE=1920, test: 38/38)
selfhost diag-parity      -> PASS
```

Not run: `verify quick` (the 60-minute CI aggregate) and `selfhost fixpoint`;
the gates that cover this change are the two above plus `verify lane`.
