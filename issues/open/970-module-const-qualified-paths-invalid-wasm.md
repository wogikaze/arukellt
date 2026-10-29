---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 970
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "numeric const modules session (2026-10-28, HEAD 2436c92b0)"
Related: "#954, #969, #966, #933"
---

# 970 — Module `const` references: qualified paths and multi-const imports emit invalid Wasm

## Summary

Const values in `std::core::{i32,i64,f64}` work when imported unqualified, but the
qualified form and some multi-const imports produce invalid modules:

```ark
use std::core::i32::{MAX}          // OK on both targets
use std::core::i32                 // module import
stdio::println(i32::MAX.to_string())  // wasm32: W0004 invalid wasm
```

So the requested Rust-like spelling `f64::INF` / `i32::MAX` is not usable yet. The
fixture `tests/fixtures/stdlib_core/const_limits.ark` currently imports names
directly as a workaround.

## Repro A (qualified path)

```ark
use std::core::i32
use std::host::stdio

fn main() {
    stdio::println(i32::MAX.to_string())
}
```

- `wasm32`: `W0004 generated Wasm failed validation: error: func 10 failed to validate
  ... type mismatch: expected i32 but nothing on stack`
- `wasm32-gc`: `compiler produced invalid core Wasm`

The const decl is registered as `SYM_FN` and typechecked through
`fn_sig_from_const_decl`, so a qualified value path appears to lower through a call to
a function whose body is never emitted (empty body, declared i32 result).

## Repro B (multi-const import)

```ark
use std::core::f64::{INF, NEG_INF, NAN}
use std::host::stdio

fn main() {
    stdio::println((INF > 1.0).to_string())
    stdio::println((NEG_INF < 0.0).to_string())
    stdio::println((NAN == NAN).to_string())
}
```

`wasm32`: invalid (func 12). Any single const or a pair (`INF, NEG_INF`) compiles and
runs. Observed on the same build at HEAD 2436c92b0 with the bootstrap compiler.

## Repro C (i64 const pair)

```ark
use std::core::i64::{MAX, MIN}
use std::host::stdio

fn main() {
    stdio::println((MAX > 0i64).to_string())
    stdio::println((MIN < 0i64).to_string())
}
```

`wasm32`: invalid module (consistent with #954, the i64 top-level const case).

## What works today

- File-scope `const X: i32 = ...` / `const INF: f64 = 1.0 / 0.0`: works on both targets.
- `use std::core::i32::{MAX}` (unqualified import) and `use std::core::f64::{INF}`:
  works on both targets.
- `use std::core::{f64, i32}` + qualified access: broken (Repro A).

## Acceptance

- [ ] `i32::MAX`, `i64::MAX`, `f64::INF` work on both targets, including multiple
      consts imported from one module.
- [ ] i64 consts work on `wasm32` (or #954 is fixed).
- [ ] `const_limits` fixture uses the qualified form once fixed.

## Implementation (2026-10-30)

Shared root with #954: const declarations are indexed as zero-argument fn
signatures, but a reference is a value.

- `src/compiler/mir/lower/core_names.ark`: qualified const paths
  (`i32::MAX`, `f64::INF`) resolve through the qualified fn index
  (`ctx_qualified_fn_index_find`) and lower like bare const names, with the
  destination local typed from the const decl's declared return value type.
  Before, a path fell through to the unresolved-path sentinel (`-1`), which
  callers treated as "no value", leaving the receiver slot empty
  (`expected i32 but nothing on stack`).
- `src/compiler/mir/lower/entry_decls.ark`: const decl bodies survive the
  stdlib AST-edge prune (a value path is not a call edge), so the const call
  target exists.
- Fixture `tests/fixtures/stdlib_core/const_reference_lowering.ark` covers the
  multi-const import form (`use std::core::f64::{INF, NEG_INF, NAN}`).
- s2 rebuilt once (`selfhost build-compiler`, ~55 s).

## Verification (2026-10-30)

- Repro A (`i32::MAX.to_string()`): `wasm32` validates and prints
  `2147483647` (exit 0). `wasm32-gc` still fails with `compiler produced
  invalid core Wasm`; on the gc path the qualified reference is still lowered
  to `nop` and the const body is pruned, so the gc MIR path never reaches the
  qualified-const branch. That needs a separate fix.
- Repro B (`use std::core::f64::{INF, NEG_INF, NAN}`): `wasm32` prints
  `true/true/false` (exit 0); `wasm32-gc` component prints `true/true/false`
  (exit 0).
- Repro C (`use std::core::i64` + `i64::MAX` / `i64::MIN`): `wasm32` prints
  `true/true/true`, with the values equal to 9223372036854775807 /
  -9223372036854775808 (exit 0). `wasm32-gc` still fails with the same invalid
  core Wasm as Repro A.
- The multi-const fixture passes on both targets.
- `tests/fixtures/stdlib_core/const_limits.ark` was not switched to the
  qualified form because the gc target is still broken.
- manifest line: `run:stdlib_core/const_reference_lowering.ark` (the Lead
  registers it; `tests/fixtures/manifest.txt` was not edited in this lane).

- [ ] `i32::MAX`, `i64::MAX`, `f64::INF` work on both targets, including multiple consts imported from one module. (wasm32 done; wasm32-gc qualified paths still invalid)
- [x] i64 consts work on `wasm32` (or #954 is fixed). (#954 is fixed; wasm32 i64 consts compile and run)
- [ ] `const_limits` fixture uses the qualified form once fixed. (blocked on the wasm32-gc failure)
