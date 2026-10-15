---
Status: done
Created: 2026-10-15
Updated: 2026-10-15
ID: 925
Track: language
Depends on: ""
Related: "#924, #825"
Orchestration class: implementation
Blocks v4 exit: False
---

# Same-line statement splitting is silent — no terminator enforcement

## Summary

`parse_block` requires nothing between statements: after a statement
ends mid-line, the remainder of the line is parsed as further
statements with no separator and no diagnostic.  Any non-infix token
following a complete statement silently splits:

    let x = 12 ~ 5      // binds x = 12; parses "~ 5" as a second
                        // expression statement and discards it
    let x = 12 ! 5      // same — x = 12, "!5" dropped
    f() g()             // two calls on one line, both kept

Code compiles and runs with silently wrong values.

## Re-diagnosis (2026-10-15)

This issue originally suspected infix `|` was not a real operator —
that was wrong.  `|` is a working infix BitOr: `12 | 5` evaluates to
13, and the byte-assembly form
`b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)` computes correctly on both
the pinned bootstrap and current s2.  The `undefined name: word`
observed in the #825 codec was the #924 newline-consumption fold
(`let v = <infix>` + `(...)` tail becoming a call), not a `|`
failure.

Operator audit on s2 (post-#924):

- `|`, `&`, `^`, `<<`, `>>` — working infix.
- `@`, `$` — lexer error (loud).
- `=>` — parse error (loud).
- `~`, `!` — **silent split** (this issue).

## Fix

Two layers, found in that order:

1. `stmt_block.ark` `parse_block`: after each statement require the
   next token to be NEWLINE, `;`, `}` or EOF — or a NEWLINE/`;`
   already consumed by the expression parser's continuation scan —
   otherwise push a parse diagnostic.  Kills the whole class uniformly
   instead of blacklisting individual tokens.
2. **Deeper root cause exposed by (1):** `lexer/number_suffix.ark`
   treated any 3-byte run starting with `i`/`u` as a type suffix.
   `255u8` followed directly by `\\n`, `+`, `)`, `,`, `]` or a
   space had that byte eaten into the literal token — `255u8\\n`
   destroyed the statement separator outright (the "phantom" the
   terminator check caught), and `255u8+1` lost its operator.  Fixed
   by validating the exact suffix set
   (i8/u8/i16/i32/i64/u16/u32/u64/f32/f64) before eating.

## Acceptance

- [x] `let x = a ~ b`, `let x = a ! b`, `f() g()` on one line are
      parse errors naming the stray token
      (`unexpected token after statement (expected newline or ';'): ~`
      at the token's span — probes verified on s2).
- [x] `x = a ~ b` (assignment stmt) errors; `if a ~ b`, `f(a ~ b)`
      already error loudly (expected-token paths) and stay loud.
- [x] Match arms, `} else {` chains, `;`-separated statements, and
      EOF/`}` termination still parse (probe + full src/compiler
      self-compile through the new parser).
- [x] compile-error fixture covering the same-line split:
      `tests/fixtures/diagnostics/same_line_statement_split.ark`
      (`.diag` = E0001, manifest `diag:` entry).
- [x] `255u8`-style 2-byte suffixes keep the following newline/operator;
      `100u8+1u8` evaluates to 101 — regression coverage folded into
      `tests/fixtures/scalar/suffix_literals.ark` (same feature area).

## Resolution

Fixed in `61b654091` (`fix(#925)`).

- `parser/stmt_block.ark`: after each statement `parse_block` requires a
  NEWLINE, `;`, `}` or EOF — or a NEWLINE/`;` already consumed by the
  expression parser's continuation scan — else a parse diagnostic naming
  the stray token's source text.
- `lexer/number_suffix.ark`: the suffix matcher now validates the exact
  suffix set and requires a non-alnum/underscore boundary; the previous
  any-3-byte match ate newlines and operators after `u8`/`i8` literals.
- `std/text/string.ark`: the single in-tree same-line assert pair split
  onto separate lines (only violation found in the full sweep).

Verified: diag-parity PASS (new `same_line_statement_split` fixture),
fixture-parity scalar 19/19 + functions 21/21 PASS, `lint --local`
over all 1931 `src/compiler/*.ark` clean, probe matrix covering every
acceptance case on the rebuilt s2. The deeper whole-tree
`compile src/compiler/main.ark` check cannot currently complete —
pre-existing hang filed as #927 (reproduces identically on the
pre-change binary).
