---
Status: open
Created: 2026-09-26
Updated: 2026-09-26
ID: 885
Parent: 852
Track: language-v2
Depends on: none
Related: \"ADR-058, RFC-011, #852, #874, #882\"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: none
Priority: 2
Source: \"#882-D generic ScanIter probe fleet — 2026-09-26\"
---

# 885 — binary + on non-Add operands compiles with divergent backend semantics

## Scope

The typechecker accepts \`+\` on operands whose type has no
\`std::core::ops::Add\` impl (only i32/i64/f64 implement it), and the
two backends then disagree on the meaning:

- wasm32-gc lowers \`String + String\` to string concat
  (\`Some(state + x)\` produced \`ab\` / \`abcc\` in the
  #882-D probe).
- wasm32 linear emits arithmetic on the String representation:
  \`String_from("x") + String_from("y")\` prints \`361\` (a
  heap address), and \`Some(a + b)\` extracts as an empty/garbage
  payload.

Same semantics across wasm32-gc and linear-memory GC is the v2 contract;
a \`+\` expression must either typecheck-error (no \`Add\` impl) or
lower to concat on both backends.

## Reproduction

```ark
fn main() {
    let c = String_from("x") + String_from("y")
    stdio::println(c)
}
```

wasm32 linear memory32 prints \`361\`; wasm32-gc prints \`xy\`.

## Acceptance

- [ ] \`String + String\` (and other non-\`Add\` operand pairs)
      is rejected at typecheck, OR \`+\` lowers to concat on both
      backends identically
- [ ] linear backend and wasm32-gc produce identical output for every
      accepted \`+\` expression

## Required evidence

- implementation commit SHA
- the reproduction program's behavior on both targets after the fix
