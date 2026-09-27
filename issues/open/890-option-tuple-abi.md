---
Status: open
Created: 2026-09-27
Updated: 2026-09-27
ID: 890
Parent: 852
Track: language-v2
Depends on: "860"
Related: "ADR-058, RFC-011, #852, #860, #888"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 2
Source: "#888 receipt — Option<(K,V)> payload shape breaks on both backends"
---

# 890 — Option<tuple> payload produces invalid wasm on both backends

## Scope

`Option<(A, B)>` — an Option whose payload is a tuple — lowers to wasm
that fails validation on both `wasm32` (linear memory) and `wasm32-gc`.
Found while probing `HashMap::entries()` consumers for #888:
`map.entries().get(0)` / `map.entries().pop()` return
`Option<(K, V)>` and hit the same shape.

Minimal reproducer (probes in `.build/probe/`, this branch):

```ark
use std::host::stdio
fn mk() -> Option<(i32, i32)> {
    Some((7, 8))
}
fn main() {
    let r = mk()
    match r {
        Some(p) => { let (a, b) = p
                     stdio::println((a + b).to_string()) }
        None => stdio::println("none")
    }
}
```

- `wasm32` (LM): wasmtime reports
  `function[10]::main ... type mismatch: values remaining on stack at
  end of block` — the Some payload unpack/destructure path leaves a
  ref value unaccounted for in the block type.
- `wasm32-gc`: core module fails `wasm-tools validate` the same way
  (`func main`, values remaining on stack at end of block).

Plain `Vec<(i32, i32)>` with `get_unchecked` + `let (a, b) = e`
destructure works on both backends (#888 verified), so the gap is
specific to the Option-with-tuple-payload ABI — likely in the
enum-payload layout / `MIR_TUPLE_LIT` payload packing or the
`Some(p)` pattern-bind destructuring path, not in tuple handling
generally.

## Acceptance

- [ ] `Option<(i32, i32)>` (and at least one `(String, i32)` pair)
      compiles, validates, and runs identically on wasm32-gc and wasm32
- [ ] `Vec<(K,V)>.get(i)` / `.pop()` on HashMap entries results work
      through `match Some((k, v))` and `let (k, v) = p`
- [ ] positive fixture on both backends

## Required evidence

- implementation commit SHA
- positive fixture + applicable compile/validate/runtime gate output
