---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 941
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-035, ADR-040, #933, #940"
---

# 941 — type-name encodings use `_` as a separator; distinct generic instances collide and miscompile

## Summary

Generic instances are encoded as strings such as
`Result_JsonValue_JsonParseError`, with `_` as the argument separator.
Identifiers may contain `_`, so the encoding is not injective:
`Result<A_B, C>` and `Result<A, B_C>` both become `Result_A_B_C`. The two
instances share one layout. On `wasm32` this silently reads the wrong field;
on `wasm32-gc` it emits invalid Wasm.

## Evidence (2026-10-27)

- `src/compiler/mir/lower/call_type_fallback.ark` L37–40 ("Underscore is a
  Result_/Option_ encoding separator, not an ident char").
- `src/compiler/wasm/sections_types_sigs_detail.ark` L203–246: competing
  spellings (`Option:`, `Option_`, `Option<`, `option:`, `option<`) for the
  same type family.

Repro (`mangle.ark`):

```ark
use std::host::stdio

struct A_B {
    x: i32,
}

struct C {
    s: String,
}

struct A {
    f: f64,
    g: f64,
}

struct B_C {
    t: String,
    u: i32,
}

fn one(k: i32) -> Result<A_B, C> {
    if k > 0 {
        return Ok(A_B { x: k })
    }
    Err(C { s: String_from("c-err") })
}

fn two(k: i32) -> Result<A, B_C> {
    if k > 0 {
        return Ok(A { f: 1.5, g: 2.5 })
    }
    Err(B_C { t: String_from("bc-err"), u: 9 })
}

fn main() {
    match one(5) {
        Ok(v) => stdio::println(v.x),
        Err(e) => stdio::println(e.s),
    }
    match two(0) {
        Ok(v) => stdio::println(v.f.to_string()),
        Err(e) => stdio::println(e.u),
    }
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`):

- `wasm32`: compiles; `wasmtime run` prints `5` then `bc-err` (expected `9`).
  This is a silent miscompile.
- `wasm32-gc`: "compiler produced invalid core Wasm".
- Control: the same program with the types renamed `AB` / `BC` prints `5`,
  `9` on both targets.

## Expected

`5`, `9` on both targets regardless of identifier spelling.

## Acceptance

- [ ] Type identity never depends on a string encoding that can collide; the
      final form uses TypeId (#933).
- [ ] Any symbol / debug name derived from a type uses an injective encoding
      (e.g. length-prefixed segments) generated from TypeId, and has exactly
      one spelling per type family.
- [ ] The repro is registered as a run fixture and passes on `wasm32` and
      `wasm32-gc`.
