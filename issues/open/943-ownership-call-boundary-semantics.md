---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 943
Track: language-design
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-002, ADR-059, ADR-060, ADR-061, ADR-088, #944, #945, #951"
---

# 943 — ownership and mutability rules do not hold at function-call boundaries

## Summary

ADR-059 makes `let` bindings immutable by default, allows mutation only
through mutable places, and rejects reuse of moved non-`Copy` values
(E0212). §7 defers the function-argument boundary to "later issues". In
practice, a function call passes non-`Copy` aggregates as implicit shared
mutable references. The callee can mutate the caller's immutable binding
through a non-`mut` parameter, and the argument is not moved.

The stdlib and the selfhost compiler rely on this behavior (`push(v, x)` on
a non-`mut` binding is the standard idiom, including the normative example
in `docs/language/memory-model.md`). Enforcing ADR-059 as written would
reject most existing code. This is a language design decision, not a local
bug.

## Evidence (2026-10-27)

Repro (`alias.ark`):

```ark
use std::host::stdio

struct Point {
    x: i32,
}

fn bump(p: Point) {
    p.x = p.x + 1
}

fn add(v: Vec<i32>) {
    push(v, 1)
}

fn main() {
    let p = Point { x: 1 }
    bump(p)
    bump(p)
    stdio::println(p.x)
    let v = Vec::new<i32>()
    add(v)
    add(v)
    stdio::println(len(v))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`): both `wasm32` and `wasm32-gc` compile and
`wasmtime run` prints `3` and `2`. The immutable `p` and `v` are mutated by
the callee, and passing them twice is accepted.

- `tests/fixtures/struct_mutation/field_assign.ark`: `fn translate(self:
  Point, …)` assigns `self.x` through a non-`mut` receiver.
- `docs/language/memory-model.md` example: `let v1 = Vec::new<i32>()`
  followed by `push(v1, 10)`.

## Decision (ADR-088, 2026-10-27)

ADR-088 adopts **implicit call-scoped borrow + explicit `mut` parameters**:

- Passing a non-`Copy` value does not move the binding; passing a moved
  binding is E0212.
- `p: T` / `self: T` are shared borrows (no mutation through them);
  `mut p: T` / `mut self: T` are mutable borrows.
- Only mutable places (`let mut` bindings, `mut` parameters / `mut self`,
  their field / element places, or temporaries) may be passed to `mut`
  parameters. Passing the same root place to a `mut` parameter and another
  parameter in one call is rejected.
- A borrowed parameter may escape (return / store / `move` capture) by
  sharing the GC reference. `Box<T>` is excluded: a borrowed `Box<T>`
  cannot be returned, stored, or `unbox`ed.
- The parameter mode is part of the function type; a shared-parameter
  function value is assignable to the `mut` form, not vice versa.

Under ADR-088 the `alias.ark` repro must be rejected: `p.x = …` through the
shared parameter of `bump`, and `push(v, 1)` on the shared parameter of
`add`. Implementation is tracked in #951; move-after-use in #944.

## Acceptance

- [x] An ACCEPTED ADR decides the model, including the receiver (`self`)
      rule and the interaction with closures (ADR-061/081) and
      `Box`/`Slice` (ADR-060): ADR-088 (ADR-059 carries a dated revision
      note pointing to it).
- [ ] ADR-059, `docs/language/memory-model.md`, `docs/language/spec.md`
      §2.4 and `docs/current-state.md` state the chosen model; no document
      claims guarantees the checker does not enforce.
- [ ] The `alias.ark` repro is registered as a fixture with the outcome the
      ADR defines (diagnostic or output), on both targets.
- [x] Follow-up implementation is tracked: #951 (parameter modes, mutable
      place and conflict checks, stdlib / selfhost migration) and #944
      (re-scoped to ADR-088 §1).
