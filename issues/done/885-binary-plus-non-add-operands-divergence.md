---
Status: done
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

The typechecker accepts \\`+\\` on operands whose type has no
\\`std::core::ops::Add\\` impl (only i32/i64/f64 implement it), and the
two backends then disagree on the meaning:

- wasm32-gc lowers \\`String + String\\` to string concat
  (\\`Some(state + x)\\` produced \\`ab\\` / \\`abcc\\` in the
  #882-D probe).
- wasm32 linear emits arithmetic on the String representation:
  \\`String_from("x") + String_from("y")\\` prints \\`361\\` (a
  heap address), and \\`Some(a + b)\\` extracts as an empty/garbage
  payload.

Same semantics across wasm32-gc and linear-memory GC is the v2 contract;
a \\`+\\` expression must either typecheck-error (no \\`Add\\` impl) or
lower to concat on both backends.

## Reproduction

\`\`\`ark
fn main() {
    let c = String_from("x") + String_from("y")
    stdio::println(c)
}
\`\`\`

wasm32 linear memory32 prints \\`361\\`; wasm32-gc prints \\`xy\\`.

## Acceptance

- [x] \\`String + String\\` (and other non-\\`Add\\` operand pairs)
      is rejected at typecheck, OR \\`+\\` lowers to concat on both
      backends identically
- [x] linear backend and wasm32-gc produce identical output for every
      accepted \\`+\\` expression

## Required evidence

- implementation commit SHA
- the reproduction program's behavior on both targets after the fix
## Progress receipt — 2026-09-26 (issue closing)

Commit: \`8f16d24a9\` lineage continues at \`30c5242ec\` on
\`docs/adr-058-language-v2-semantics\` (this receipt).

Fix direction per ADR-065 section 2: \`String\` has no \`Add\` impl, so
\`+\` must be a type error — the typechecker previously bypassed the
trait-impl check for every scalar plus String, letting the two
backends pick different meanings. \`is_builtin_operator_type\` is now
per protocol family: Eq/Ord keep all scalars + String on the
intrinsic comparison path, Not keeps bool + i32/i64, bit ops keep
i32/i64, and the arithmetic family (Add/Sub/Mul/Div/Rem/Neg) is
restricted to numeric scalars so String/bool/char reach the Add
registry and error when no impl exists.

Evidence:

- Implementation commit \`30c5242ec\` (\`operator_traits.ark\` per-trait
  bypass + \`names::type_display_name\` for the operand name in the
  error message).
- Reproduction: \`String_from("x") + String_from("y")\` now errors
  \`E0208: operator \`+\` requires Add for String\` identically on
  wasm32 linear memory32 and wasm32-gc component (previously: 361 on
  linear, "xy" on gc).
- Same-family probes: \`true + false\` errors \`Add for bool\`;
  \`String - String\` errors \`Sub for String\` — identical codes on
  both targets.
- Accepted-\`+\` parity: \`3 + 4\` prints 7, \`1.5 + 2.5\` numeric,
  \`eq(String, String)\`/\`!\`/\`&\`/\`<<\` all produce identical results on
  wasm32 linear; fixture \`fn_zero_value_positions\`,
  \`option_fn_call\`, \`vec_fn_push_call\`, \`hashmap_i32_fn_zero_call\`,
  \`escape_store\` still pass on both targets (gc component
  wasm-tools validate OK).
- New diag fixture \`diagnostics/operator_add_no_impl\` locks the
  \`E0208\` message; \`verify lane\` PASS (editorconfig, quality
  contract 24 rules, lint ratchet).

Residual (filed as #886): \`impl Add for Point\` in-file satisfies
\`a + b\` only when \`std::core::ops\` is imported — a pre-existing
import-visibility gap in \`type_satisfies_trait_bound\` present since
canonical-trait routing (#860), confirmed identical on an s2 built
without this change. \`diagnostics/operator_overload.ark\` fails for
that pre-existing reason, not for this fix.
