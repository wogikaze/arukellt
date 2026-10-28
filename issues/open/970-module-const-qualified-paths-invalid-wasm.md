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
