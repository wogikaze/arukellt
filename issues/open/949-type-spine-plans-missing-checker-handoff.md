---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 949
Track: docs
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-040, ADR-053, RFC-002, #851, #933"
---

# 949 — type-spine plans omit the typechecker → CoreHIR handoff; a core-rewrite checkbox overstates progress

## Summary

The two executable plans that are supposed to remove type inference from the
backend do not include the step that makes it possible: the typechecker
exporting expression / local types and resolved call targets (#933, #934).
One completed checkbox also claims more than the code shows.

## Evidence (2026-10-27)

- `docs/plans/typed-mir-signature-registry.md` Phase 1 registers only
  typechecker function signatures into TypeTable / SignatureRegistry.
  Phase 3 ("Typed MIR") has lowering set MirValueType itself. No phase moves
  per-expression types or call resolution out of the checker.
- `docs/plans/selfhost-compiler-core-rewrite.md` Phase 2 marks
  "[x] hot MIR から `type_name: String` を外し、判定は TypeId" as done.
  `MirLocal` still carries `type_name`, the post passes decide on it
  (`src/compiler/mir/post_pass_callee_lookup.ark`,
  `src/compiler/mir/post_pass_type_propagate.ark`), and
  `rg -c type_name src/compiler/mir` is ≈ 1990 vs ≈ 78 for `TypeId`.

## Acceptance

- [ ] `typed-mir-signature-registry.md` gains a phase (before Phase 3) for the
      checker exporting node-keyed TypeIds and resolved FunctionIds, with
      #933 / #934 as owners.
- [ ] `selfhost-compiler-core-rewrite.md` Phase 2's `type_name` checkbox is
      reopened or narrowed to what is true (instruction storage only), with
      the `MirLocal` remainder pointing to Phase 4 / #933.
- [ ] Phase 4 ("post-pass type propagation を消す") references #933 as the
      source-of-types prerequisite.
- [ ] Docs-only change: no fixture change is needed; `python3 scripts/manager.py
      verify quick` stays green.
