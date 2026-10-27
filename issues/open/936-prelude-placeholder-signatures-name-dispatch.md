---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 936
Track: stdlib
Depends on: "#934"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#934"
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-042, ADR-046, RFC-005, #917, #928, #933, #935"
---

# 936 — prelude core functions have placeholder signatures; checker and emitter dispatch them by name

## Summary

Core collection / string functions in `std/prelude.ark` declare signatures
that do not describe their real contract, and their bodies are
`panic(...)`. The comment says "polymorphic in practice; emitter dispatches
by name". The typechecker special-cases these names and skips argument
checking; the emitter recognizes the names and emits intrinsics. As a
result, ill-typed calls are accepted and become memory reinterpretation on
`wasm32` and invalid Wasm on `wasm32-gc`.

## Evidence (2026-10-27)

- `std/prelude.ark` L663–700: `pub fn len(s: String) -> i32`,
  `pub fn push(v: Vec<i32>, val: i32)`, `pub fn get(v: Vec<i32>, idx: i32)`,
  `pub fn is_empty(v: Vec<i32>)`, each with a `panic(String_new())` body.
  L79–84: `pub fn clone(s: String) -> String` is used for every type (#928).
- `src/compiler/typechecker/builtins.ark` `builtin_call_return_type`
  returns `i32` for any `len(...)` and `TY_UNKNOWN` for unknown names; the
  arguments are not checked.

Repro 1 (`ty_push.ark`):

```ark
use std::host::stdio

fn main() {
    let v = Vec::new<i32>()
    push(v, String_from("oops"))
    stdio::println(len(v))
    let x: i32 = get_unchecked(v, 0)
    stdio::println(x)
}
```

Repro 2 (`ty_len.ark`):

```ark
use std::host::stdio

fn main() {
    let n = 42
    stdio::println(len(n))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`):

- `wasm32`: both compile; `wasmtime run` prints `1` / `140` (the string
  pointer read as `i32`) and `0`.
- `wasm32-gc`: both report "compiler produced invalid core Wasm".

## Expected

`push(Vec<i32>, String)` and `len(i32)` are compile-time type errors. Core
operations have real signatures (generic functions or trait methods per
ADR-046), and backend lowering is bound to them through FunctionId →
CoreOp (ADR-042), not through the call-site spelling.

## Acceptance

- [ ] No prelude function declares a placeholder signature; the remaining
      `panic` placeholder bodies are replaced by real bodies or by sealed
      CoreOp bindings.
- [ ] The typechecker has no name-keyed return-type or argument exemptions
      for these operations (`builtins.ark`, `call_method.ark`,
      `vec_core_mono.ark`, `hashmap_methods.ark` name lists removed or made
      signature-driven).
- [ ] The emitter selects intrinsics through the resolved FunctionId / CoreOp,
      not by callee-name prefix (`starts_with(callee, "push__")` etc.).
- [ ] Compile-error fixtures for both repros pass on `wasm32` and
      `wasm32-gc`.
- [ ] #928 (`clone(Vec<T>)` on `wasm32-gc`) is fixed by the same change.

## Notes

#917 is converting stdlib free functions to methods; this issue owns the
type-contract part (signatures must be true) so that #935 can be enabled.
