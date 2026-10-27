---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 944
Track: typecheck
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-059, ADR-088, E0212, #943, #945, #951"
---

# 944 — use of a moved binding is not diagnosed when the use is a call argument

## Summary

ADR-059 §4: reuse of a moved non-`Copy` binding is E0212. Move tracking only
covers particular AST shapes. `consume_expr` returns early for method calls,
paths, blocks, binaries, and so on, and for plain calls it only consumes the
`Box_new` / `unbox` boundary. Identifier inference marks the binding as used
but does not check its moved state. After `let b = a`, passing `a` to any
function is accepted, and `a` and `b` alias.

## Evidence (2026-10-27)

- `src/compiler/typechecker/ownership.ark` L46–96 (`consume_expr`; `NK_CALL`
  → `consume_box_boundary_argument` only).
- `src/compiler/typechecker/infer_leaf.ark` L40–50 (identifier inference
  does not consult moved state).

Repro 1 (`ext_move.ark`):

```ark
use std::host::stdio

struct P {
    s: String,
}

fn show(p: P) {
    stdio::println(p.s)
}

fn main() {
    let a = P { s: String_from("a") }
    let b = a
    show(a)
    show(b)
}
```

Repro 2 (`move_let.ark`):

```ark
use std::host::stdio

fn main() {
    let v1 = Vec::new<i32>()
    let v2 = v1
    push(v1, 1)
    stdio::println(len(v2))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`): both compile on `wasm32` and `wasm32-gc`;
`wasmtime run` prints `a`, `a` and `1`. No E0212 is reported.

## Expected

ADR-059 §4 and ADR-088 §1 (decided 2026-10-27 for #943): passing a moved
binding as an argument is E0212, like any other use. Both repros are E0212
at the use of the moved binding (`show(a)`, `push(v1, 1)`). Passing a
non-moved binding is a call-scoped borrow and does not move it.

## Acceptance

- [ ] Binding state (moved / Copy / borrowed) is checked at every use of an
      identifier in one place, not per AST shape.
- [ ] Both repros are registered compile-error fixtures (E0212) on both
      targets.
- [ ] Existing ownership / closure-capture fixtures (E0212–E0214) stay
      green.
- [ ] `python3 scripts/manager.py selfhost fixpoint` stays green (selfhost
      sources migrated if the rule tightens).
