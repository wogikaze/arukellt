---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 943
Track: language-design
Depends on: none
Orchestration class: design-needed
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-002, ADR-059, ADR-060, ADR-061, #944, #945"
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

## Decision required (ADR)

Pick one model and supersede / amend ADR-059 accordingly:

- (a) **Ownership at calls**: non-`Copy` arguments move by default;
  mutation requires an explicit parameter mode (`mut` / borrow form). This
  is consistent with ADR-059/060, but requires migrating the whole stdlib
  and compiler and conflicts with ADR-002's no-borrow-checker goal.
- (b) **Reference semantics**: non-`Copy` aggregates are shared references;
  `let` immutability means "no rebinding" only; E0212 / unique-ownership
  claims for `Box` are withdrawn or narrowed. This matches current code.
  Optionally, add a cheap rule that a callee must declare a parameter
  `mut` to mutate through it.

## Acceptance

- [ ] An ACCEPTED ADR decides (a), (b), or another model, including the
      receiver (`self`) rule and the interaction with closures (ADR-061/081)
      and `Box`/`Slice` (ADR-060).
- [ ] ADR-059, `docs/language/memory-model.md`, `docs/language/spec.md`
      §2.4 and `docs/current-state.md` state the chosen model; no document
      claims guarantees the checker does not enforce.
- [ ] The `alias.ark` repro is registered as a fixture with the outcome the
      ADR defines (diagnostic or output), on both targets.
- [ ] Follow-up implementation is tracked (#944 is re-scoped to the
      decision).
