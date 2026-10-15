---
Status: open
Created: 2026-10-15
Updated: 2026-10-15
ID: 925
Track: language
Depends on: ""
Related: "#924, #825"
Orchestration class: implementation
Blocks v4 exit: False
---

# Infix `|` silently terminates a `let` RHS — remainder parsed as garbage

## Summary

Ark lexes `|` as TK_PIPE but does not accept it as an infix expression
operator.  Worse, it fails *silently*: `let word = b0 | (b1 << 8)`
binds `word = b0` and the trailing `| (b1 << 8)` is consumed as
separate garbage statements with no diagnostic.  Code compiles and runs
with silently wrong values.

Verified empirically (#825, 2026-10-15): `&`, `>>`, `^` infix work;
`|` does not (statement truncation).  Canonical bitwise ops are the
trait methods `.bitand/.bitor/.bitxor/.shl/.shr` (std::core::ops).

## Evidence

- Minimal repro (deleted; was .build/minrepro_825a.ark):
      fn rd(data: String, pos: i32) -> (i32, i32) {
          let word = b0 | 1
          (word, pos)
      }
  → `error[E0100|resolve]: undefined name: word` at the tail line —
  proving `let word = b0 | 1` never bound `word` through the intended
  RHS.
- The original AST cache codec's `let val = b0 | (b1<<8) | (b2<<16) |
  (b3<<24)` silently computed `val = b0` — every i32 read returned only
  the low byte.  This was a contributing cause of the cache being
  disabled as broken (#825).

## Decision needed

Either:
1. make `|` a real infix BitOr operator (it lexes; parse+typecheck it
   like `&`), or
2. reject it with a hard parse error ("use .bitor()"), never silent
   truncation.

Silent miscompile is the worst option and is the current behavior.

## Acceptance

- [ ] `let x = a | b` either evaluates BitOr or is a parse error —
      no silent split.
- [ ] Same for assignment/expr-statement positions (`x = a | b`,
      `if a | b`, arg positions).
- [ ] Regression fixture covering `|` in let/assign/if positions.
- [ ] Audit other non-operator tokens that lex but are not infix
      expressions for the same silent-truncation class.
