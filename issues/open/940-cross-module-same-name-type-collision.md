---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 940
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-035, ADR-040, #931, #933, #941, #942"
---

# 940 — backend type identity drops module paths; same-named types in different modules collide

## Summary

Lowering and layout tables identify nominal types by bare name. Module
qualifiers are stripped (`strip_module_double_colon`,
`strip_embedded_module_path_prefixes`). Two modules that each define
`pub struct Item` get one layout, and field access / construction of one
uses the other's layout.

## Evidence (2026-10-27)

- `src/compiler/mir/lower/call_type_fallback.ark` L17–40: "Layout /
  local-type tables use bare struct names. Strip only `::` qualifiers".

Repro (`samename/`):

```ark
// a.ark
pub struct Item {
    name: String,
    count: i32,
}

pub fn make(n: i32) -> Item {
    Item { name: String_from("alpha"), count: n }
}

pub fn describe(it: Item) -> String {
    it.name
}
```

```ark
// b.ark
pub struct Item {
    weight: f64,
    id: i64,
    tag: i32,
}

pub fn make(t: i32) -> Item {
    Item { weight: 2.5, id: 99, tag: t }
}

pub fn tag_of(it: Item) -> i32 {
    it.tag
}
```

```ark
// main.ark
use std::host::stdio
import a
import b

fn main() {
    let x = a::make(3)
    let y = b::make(7)
    stdio::println(a::describe(x))
    stdio::println(b::tag_of(y))
    stdio::println(x.count)
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`):

- `wasm32`: "compilation succeeded", but the module is invalid
  (`wasmtime` run: `function[11]::make … type mismatch: expected i32, found
  f64`). See #942 for the missing validation.
- `wasm32-gc`: "compiler produced invalid core Wasm".

## Expected

Prints `alpha`, `7`, `3` on both targets. Nominal type identity is
(module path, item name, type arguments).

## Acceptance

- [ ] Type keys in lowering / layout / GC type tables include the defining
      module path; no `::`-stripping on type identity.
- [ ] The repro is registered as a run fixture under `tests/fixtures/modules/`
      and passes on `wasm32` and `wasm32-gc`.
- [ ] A second fixture covers same-named enums and same-named generic structs
      across modules.
- [ ] Final form uses TypeId from #933; any interim key change must be a
      correct identity (not a heuristic) per the semantic-debt rule.

## Notes

The selfhost compiler is built through the flat-src overlay (#931), which
merges namespaces, so its own fixpoint never exercises cross-module type
identity. #950 tracks the verification gap.
