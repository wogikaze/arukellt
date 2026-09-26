---
Status: done
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

- [x] \`impl Add for Point\` in-file satisfies \`a + b\` with and
      without \`use std::core::ops\` (identical output on wasm32
      linear and wasm32-gc)
- [x] \`diagnostics/operator_overload.ark\` passes its run/t3
      fixture contract again
- [x] No regression on operator diag fixtures
      (\`operator_add_no_impl\` and friends keep erroring for types
      with no impl)

## Required evidence

- implementation commit SHA
- reproduction program output on both targets with and without the
  ops import
- fixture-test result for the diagnostics directory

## Progress receipt — 2026-09-26 (issue closing)

Commit: \`23658247e\` on \`docs/adr-058-language-v2-semantics\`.

Root cause: \`type_satisfies_trait_bound_at_depth\` in
\`trait_bounds.ark\` used \`depth >= len(env.trait_defs)\` as
fuel against cyclic supertrait chains. With an empty trait registry —
the case whenever no file imports \`std::core::ops\` and no trait
is declared locally — the top-level call hit \`0 >= 0\` and
returned false even though a matching impl was already registered in
\`env.trait_impls\`. Importing \`std::core::ops\` populated
\`trait_defs\`, the guard let the match through, and the impl was
found — that is the entire import dependence, not a resolver gate.
The guard is now \`depth > len(env.trait_defs)\`, which preserves
the cycle bound (a simple supertrait path has at most trait_defs
nodes) without rejecting the top-level call on an empty registry.

Evidence:

- Implementation commit \`23658247e\` (\`trait_bounds.ark\`
  guard relaxation only; debug instrumentation removed before commit).
- Reproduction program: without \`use std::core::ops\`, \`a + b\`
  on \`impl Add for Point\` now compiles and prints \`4\` on
  both wasm32 linear memory32 and wasm32-gc component; with the
  import it prints \`4\` identically on both targets.
- \`diagnostics/operator_overload.ark\`: wasm32 run prints
  \`4\` / \`6\` matching \`.expected\`; wasm32-gc component
  compiles and runs to the same output (run + t3-compile + t3-run
  contract).
- Operator diag regression: \`operator_add_no_impl\` still errors
  E0208 for \`Add\` on String; \`operator_trait_missing\` still
  errors for Point (inherent method is not a trait impl) — both match
  their \`.diag\` expectations.
- Neighbouring fixtures: \`operator_overload/add_complex\`,
  \`neg_wrapper\`, \`sub_vec\`, \`index_custom\`,
  \`trait/operator_overload\`, \`stdlib_trait/operator_generic\`
  compile and run on wasm32 (wasm32-gc where probed).
- \`python3 scripts/run/selfhost-fixture-test.py --filter-dir
  diagnostics\`: PASS=5 FAIL=0 (5 compile+validate, 1 runtime smoke).
- \`python3 scripts/manager.py verify lane\`: PASS (comment policy,
  semantic-debt, lint smoke, touched-code ratchet, editorconfig,
  quality contract, fmt, lint ratchet).

Scope note: the same guard also gated every other
\`type_satisfies_trait_bound\` caller (for/iterator bounds,
type-parameter bounds, method resolution) under an empty trait
registry; the fix is general, not operator-specific.
