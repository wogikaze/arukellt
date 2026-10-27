---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 945
Track: typecheck
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-059, ADR-060, #943, #944, #946"
---

# 945 — `impl Copy` is accepted for types with non-`Copy` fields; the "copy" aliases

## Summary

A user type counts as `Copy` whenever an `impl Copy for T {}` exists. The
fields are never checked. A struct holding a `Vec` can be declared `Copy`.
"Copying" it then shares the `Vec`, so a mutation through the copy is
visible through the original. This breaks ADR-059's guarantee that
copies are independent values.

## Evidence (2026-10-27)

- `src/compiler/typechecker/ownership_types.ark` L52–64
  (`has_explicit_copy_impl` only searches for the impl).
- `src/compiler/typechecker/trait_registry.ark` L186–244 (impl registration
  performs no structural `Copy` check).

Repro (`ext_copy2.ark`):

```ark
use std::host::stdio

struct Holder {
    v: Vec<i32>,
}

impl Copy for Holder {}

fn main() {
    let a = Holder { v: Vec::new<i32>() }
    let b = a
    push(b.v, 1)
    stdio::println(len(a.v))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`): accepted on `wasm32` and `wasm32-gc`;
`wasmtime run` prints `1`.

## Expected

`impl Copy for Holder` is rejected because `Vec<i32>` is not `Copy`. For
generic types, the impl is valid only when the field types are `Copy` under
the impl's bounds.

## Acceptance

- [ ] `impl Copy for T` validates that every field (struct) / payload (enum)
      type is `Copy`, recursively and under generic bounds; violation is a
      dedicated diagnostic at the impl.
- [ ] The repro is a compile-error fixture on both targets;
      `tests/fixtures/ownership/copy_and_mutability.ark` stays green.
- [ ] stdlib / selfhost sources with invalid `Copy` impls are fixed.
