---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 969
Track: language-design
Depends on: none
Orchestration class: design-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: "numeric limits / VecDeque session (2026-10-28, HEAD 0a64bdb62)"
Related: "#954, #966, ADR-046, ADR-072"
---

# 969 — Rust-style numeric constants (`i32::MAX`, `f64::INFINITY`) and type aliases (`VecDeque<T>`) are not expressible

## Summary

The requested Rust-like surface cannot be written today:

| Want | Current state |
| --- | --- |
| `i32::MAX`, `i64::MAX`, `f64::INFINITY` (associated const) | no associated-const syntax |
| `i32::MAX()` (associated fn on a primitive) | lower to an unreachable stub; calling traps |
| module-level `pub const I32_MAX: i32 = ...` | referencing it fails `wasm32` validation (extends #954, which was i64-only) |
| `type VecDeque<T> = Deque<T>` | `type` alias syntax is a parse error |

## Repro A (`i32::MAX()`)

```ark
impl i32 {
    fn MAX() -> i32 { 2147483647 }
}

fn main() {
    // compile OK; runtime: wasm trap: wasm `unreachable` instruction executed
    __print_i32(i32::MAX())
}
```

The same happens for `i64::MAX()` / `f64::INFINITY()` (`1.0 / 0.0`).

## Repro B (top-level const)

```ark
pub const I32_MAX: i32 = 2147483647
```

Referencing `I32_MAX` from `main` compiles on `wasm32-gc` but the `wasm32` backend
emits an invalid module (`W0004 ... func N failed to validate`). #954 reported the
i64 case; i32 and f64 consts behave the same.

## Repro C (type alias)

```ark
type Ints = Vec<i32>
```

`error[E0001|parse]: unexpected token in expression: 10` (the alias is parsed as an
expression).

## Shipped workaround

`std/core/limits.ark` provides module functions and works on both targets:

```ark
use std::core::limits
let inf: i32 = limits::i32_max()
let pinf: f64 = limits::f64_infinity()
```

Fixture: `tests/fixtures/stdlib_core/num_limits.ark` (registered as `run:`).

## Acceptance

- [ ] Associated consts (or at least `i32::MAX()`-style associated functions) work on both targets.
- [ ] Top-level const references validate on `wasm32` (fix #954 for i32/f64 too).
- [ ] `type` aliases parse, enabling `VecDeque<T> = Deque<T>` and similar naming compatibility.

## Update (2026-10-28, later session)

Const values now exist as module-level `const` decls instead of functions:

- `std/core/i32.ark`: `MAX`, `MIN`
- `std/core/i64.ark`: `MAX`, `MIN`
- `std/core/f64.ark`: `INF`, `NEG_INF`, `NAN`

They work when imported unqualified (`use std::core::i32::{MAX}`) on both targets.
The Rust-like qualified spelling (`i32::MAX`, `f64::INF`) still emits invalid Wasm, and
multi-const imports have their own failure, both tracked in #970. Associated-const
syntax and type aliases remain unsupported.
