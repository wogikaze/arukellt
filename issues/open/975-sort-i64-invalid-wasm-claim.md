---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 975
Track: compiler
Depends on: 953
Orchestration class: verification-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: "ABC481 arukellt port review (2026-10-28, HEAD 2436c92b0; deliverable README claim)"
Related: "#953, #965, #971, #972"
---

# 975 — `std::collections::sort::sort` on `Vec<i64>`: ABC481 reported invalid Wasm on `wasm32`; current build traps instead

## Summary

The ABC481 port records that `std::collections::sort::sort` produces invalid Wasm for
`Vec<i64>` on `wasm32`, which is why E hand-writes a merge sort:

> `std::collections::sort::sort` は Vec<i64> に対して不正な wasm を生成する。

On the current build the free `sort` path fails earlier than validation:

```ark
use std::collections::sort

fn main() {
    let v: Vec<i64> = Vec::new<i64>()
    v.push(3i64)
    v.push(1i64)
    sort::sort(v)
}
```

- `wasm32`: compiles to a **valid** module (`wasm-tools validate` passes), then traps
  at runtime inside `is_less__i64`.
- `wasm32-gc`: compiles, then traps inside `is_less__i64`.
- `sort::sort(Vec<i32>)`, `Vec<String>`, and `Vec<i64>` all behave this way. That is
  the `Ord::cmp` stub/trap class tracked as #953, not a validation failure.

So either the recorded "invalid Wasm" was a pre-#953-fix observation, or there is a
different monomorphization path (call site, module function, gc lowering) that still
emits invalid Wasm. This issue exists to settle that distinction; if #953 covers it,
close this as duplicate.

## Acceptance

- [ ] Identify the recorded `Vec<i64>` sort case (target, call site, compiler build)
      and reproduce the invalid-Wasm output, or confirm it was the #953 runtime trap.
- [ ] If distinct: minimal repro + fix, with `wasm-tools validate` on the emitted module.
- [ ] If covered by #953: fold the evidence into #953 and close this issue.
- [ ] `sort::sort` on `Vec<i32>` and `Vec<i64>` sorts correctly on both targets
      (shared with #953 acceptance).

## Notes

- #965 covers generic `<` on `T: Ord` emitting invalid Wasm on `wasm32-gc`, which is
  the most likely codegen root behind the recorded observation.
- #971/#972 cover aggregate-element `sort_by` and `(a..b).rev()` invalid-Wasm cases.
- Probe files: `tmp/issues/probe_sort32.ark`, `probe_sort64.ark`, `probe_sort_n.ark`.
