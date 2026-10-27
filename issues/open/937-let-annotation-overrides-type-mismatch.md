---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 937
Track: compiler
Depends on: "#934"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#934"
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "#815, #933, #935, #938"
---

# 937 — `let` type annotations silently replace a mismatching inferred type

## Summary

When the inferred initializer type and the annotation share a constructor
(`Vec`, `Option`, enum, struct) but differ in a type argument,
`should_prefer_annotation` returns true and the binding takes the
annotation without unifying. A `Vec<i32>` can be bound as `Vec<String>`
with no diagnostic.

## Evidence (2026-10-27)

- `src/compiler/typechecker/stmt_let.ark` L43–58 (annotation override, with
  only comprehensions excluded since #815) and L208–237
  (`should_prefer_annotation`: returns true when any type-argument tag
  differs).

Repro (`ext_let.ark`):

```ark
use std::host::stdio

fn ints() -> Vec<i32> {
    let v = Vec::new<i32>()
    push(v, 7)
    v
}

fn main() {
    let v: Vec<String> = ints()
    stdio::println(get_unchecked(v, 0))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`):

- `wasm32`: compiles; `wasmtime run` prints an empty line (the `i32` 7 read
  as a string).
- `wasm32-gc`: "compiler produced invalid core Wasm".

## Expected

E0200 `Vec<i32>` vs `Vec<String>` at the `let`, on both targets.

## Acceptance

- [ ] `should_prefer_annotation` is removed; annotations are always unified
      with the inferred type through the single type-compatibility routine
      shared with #935 / #938.
- [ ] The weak-inference cases the override was hiding (e.g. `from_iter`
      picking the first impl by name) are fixed at their source (#934), not
      re-allowlisted.
- [ ] Compile-error fixture for the repro on `wasm32` and `wasm32-gc`.
- [ ] `python3 scripts/manager.py selfhost fixpoint` stays green.
