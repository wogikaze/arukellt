---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 944
Track: typecheck
Depends on: "#943"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#943"
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-059, E0212, #943, #945"
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

Under the current ADR-059 text, both repros are E0212 at the second use of
the moved binding. The final rule follows the #943 decision. Even under
reference semantics, `let b = a` must not leave a "moved" binding usable
while documentation says it is moved.

## Acceptance

- [ ] Binding state (moved / Copy / borrowed) is checked at every use of an
      identifier in one place, not per AST shape.
- [ ] Both repros are registered fixtures with the outcome defined by #943,
      on both targets.
- [ ] Existing ownership / closure-capture fixtures (E0212–E0214) stay
      green.
- [ ] `python3 scripts/manager.py selfhost fixpoint` stays green (selfhost
      sources migrated if the rule tightens).
