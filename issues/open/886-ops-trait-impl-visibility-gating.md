---
Status: open
Created: 2026-09-26
Updated: 2026-09-26
ID: 886
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-065, RFC-011, #852, #885, #860"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: none
Priority: 2
Source: "#885 fixture regression sweep — 2026-09-26"
---

# 886 — local trait impls only satisfy operator checks when std::core::ops is imported

## Scope

ADR-065 section 2 requires that the presence or absence of
\`use std::core::ops\` must not change the legality or meaning of
\`a + b\`. Today it does: a user-defined \`impl Add for Point\` in
the same file satisfies the operator check only when the file also
imports \`std::core::ops\`. Without the import the impl registers
(text name \`Add\`) but \`type_satisfies_trait_bound\` does not
find it, so \`a + b\` is rejected with
\`operator \`+\` requires Add for Point\`.

## Reproduction

\`\`\`ark
struct Point {
    x: i32,
    y: i32,
}

impl Add for Point {
    fn add(self: Point, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
}

fn main() {
    let a = Point { x: 1, y: 2 }
    let b = Point { x: 3, y: 4 }
    let c = a + b
    stdio::println(c.x)
}
\`\`\`

Fails with \`E0208: operator \`+\` requires Add for Point\` on both
targets; adding \`use std::core::ops\` compiles and prints 4.
Pre-existing since the #860 canonical-trait routing (verified on s2
built from the pre-#885 typechecker): the
\`diagnostics/operator_overload.ark\` run/t3 fixture has been failing
this check, surfacing only under the #885 sweep.

## Acceptance

- [ ] \`impl Add for Point\` in-file satisfies \`a + b\` with and
      without \`use std::core::ops\` (identical output on wasm32
      linear and wasm32-gc)
- [ ] \`diagnostics/operator_overload.ark\` passes its run/t3
      fixture contract again
- [ ] No regression on operator diag fixtures
      (\`operator_add_no_impl\` and friends keep erroring for types
      with no impl)

## Required evidence

- implementation commit SHA
- reproduction program output on both targets with and without the
  ops import
- fixture-test result for the diagnostics directory

