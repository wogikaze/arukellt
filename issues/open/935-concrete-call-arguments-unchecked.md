---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 935
Track: compiler
Depends on: "#934, #936"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#934, #936"
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-015, ADR-040, #933, #937, #938, #939"
---

# 935 — arguments of non-generic calls are never checked against parameter types

## Summary

`infer_concrete_call_expr` checks only the argument count. It infers the
argument expressions but does not unify them with the callee's parameter
types. Any value can be passed to any concrete parameter. On `wasm32` the
result is a memory fault or garbage; on `wasm32-gc` it is invalid core Wasm.
Generic calls do unify, so concrete functions are less safe than generic
ones.

## Evidence (2026-10-27)

Code: `src/compiler/typechecker/call_fn.ark` L131–158
(`call_args::infer_call_args(...)` then `FnSig_return_type(sig)`, no unify).

Repro (`ext_args.ark`):

```ark
use std::host::stdio

struct P {
    s: String,
    n: i32,
}

fn add_one(x: i32) -> i32 {
    x + 1
}

fn name_of(p: P) -> String {
    p.s
}

fn main() {
    stdio::println(add_one(true))
    stdio::println(name_of(42))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`, built 2026-10-27):

- `--target wasm32`: "compilation succeeded"; `wasmtime run` traps with
  `memory fault at wasm address 0xfffffffc` / out of bounds memory access.
- `--target wasm32-gc --emit component`: "compiler produced invalid core
  Wasm".

## Expected

`add_one(true)` and `name_of(42)` are E0200 type mismatches at the argument
span, identical on both targets.

## Acceptance

- [ ] Concrete call arguments are unified with the resolved signature from
      #934 (same unify as generic calls).
- [ ] `tests/fixtures/` gains a compile-error / `.diag` fixture for the repro
      above, checked on `wasm32` and `wasm32-gc`.
- [ ] Prelude / stdlib bodies type-check without per-name exemptions (needs
      #936).
- [ ] `python3 scripts/manager.py verify fixtures` and
      `python3 scripts/manager.py selfhost fixpoint` stay green.

## Notes

Adding argument unification before #934/#936 will false-fail stdlib bodies
(per the code comment). Do not land a per-name allowlist to get around that.
