---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 938
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "#933, #935, #937, #939"
---

# 938 — return-type checking compares only the outer tag, not type arguments

## Summary

`check_return_type_compat` compares the return and value type tags (plus a
struct/enum name special case) and returns early when either side is
unknown. It never compares type arguments, so a function declared
`-> Vec<String>` may return a `Vec<i32>`.

## Evidence (2026-10-27)

- `src/compiler/typechecker/returns.ark` L7–49: tag comparison only;
  `TY_UNKNOWN` on either side returns without error; `i32`/`char` and
  struct/enum same-name pairs are accepted.

Repro (`ext_ret.ark`):

```ark
use std::host::stdio

fn strs() -> Vec<String> {
    let v = Vec::new<i32>()
    push(v, 7)
    v
}

fn main() {
    let v = strs()
    stdio::println(get_unchecked(v, 0))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`):

- `wasm32`: compiles; `wasmtime run` prints an empty line.
- `wasm32-gc`: "compiler produced invalid core Wasm".

## Expected

E0200 return type mismatch `Vec<String>` vs `Vec<i32>` on both targets.

## Acceptance

- [ ] Return values are checked with the same full, recursive type
      compatibility routine used for `let` annotations and call arguments
      (one definition of "same type" in the checker).
- [ ] The implicit `i32`/`char` and struct/enum same-name acceptances are
      either removed or justified by an ACCEPTED ADR and covered by fixtures.
- [ ] Compile-error fixture for the repro on `wasm32` and `wasm32-gc`.
- [ ] `python3 scripts/manager.py selfhost fixpoint` stays green.
