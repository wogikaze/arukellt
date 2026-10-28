---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 966
Track: compiler
Depends on: none
Orchestration class: architecture-implementation
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "Vec/stdlib surface expansion session (2026-10-28, HEAD 0a64bdb62)"
Related: "#933, #936, #940, #965, ADR-059"
---

# 966 — Same-named methods across type specializations resolve to one impl: `Vec::contains` traps and `Vec<String>::sort` runs the i32 sort

## Summary

Method resolution is keyed on the method name, not on the receiver's specialized impl.
Consequences observed while adding the Vec surface (`std/collections/vec.ark`):

1. `Vec<i32>.contains(x)` traps with `unreachable`, while a renamed copy
   (`has_dbg`) in the same impl works. The name likely resolves to the prelude
   `contains(String, String)` helper or the iterator `contains` adapter.
2. `Vec<String>.sort()` runs the **i32** insertion-sort body: the vector is left
   unchanged, and `Vec<String>.dedup/lower_bound/upper_bound` are equally wrong.
3. Adding both `impl Vec<i32>` and `impl Vec<String>` blocks with the same method
   names is accepted by the typechecker but only one body is selected.

## Repro (contains)

```ark
use std::collections::vec
use std::host::stdio

fn main() {
    let v: Vec<i32> = Vec::new<i32>()
    v.push(1)
    v.push(3)
    stdio::println(v.contains(3).to_string())   // traps: wasm `unreachable`
}
```

Renaming the method to `has_dbg` (same body) prints `true`.

## Repro (String sort)

```ark
use std::collections::vec
use std::host::stdio

fn main() {
    let v: Vec<String> = Vec::new<String>()
    v.push(String_from("pear"))
    v.push(String_from("apple"))
    v.push(String_from("fig"))
    v.sort()
    stdio::println(v.get_unchecked(0))   // prints "pear" (unchanged)
}
```

## Workarounds used

- `contains` was intentionally not added to the shipped Vec surface until this is
  fixed (the canonical name traps).
- String ordering methods were withdrawn; only the type-generic mutation/access
  methods (`swap`/`reverse`/`insert`/`remove`/`swap_remove`/`first`/`last`) and the
  i32 comparison methods are shipped.

## Acceptance

- [ ] `Vec<i32>.contains` works without renaming.
- [ ] `Vec<String>.sort/dedup/lower_bound/upper_bound` run the String impl.
- [ ] Fixture: same-named methods on `Vec<i32>` and `Vec<String>` in one program.
