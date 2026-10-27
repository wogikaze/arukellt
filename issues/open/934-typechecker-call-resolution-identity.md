---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 934
Track: compiler
Depends on: none
Orchestration class: architecture-implementation
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-040, ADR-042, ADR-046, #933, #935, #936"
---

# 934 — typechecker resolves calls through a flat, overload-blind name index

## Summary

Function signatures are looked up by flat (module-stripped) name in
`src/compiler/typechecker/sig_lookup.ark`. The index cannot distinguish
same-named functions in different modules or stdlib bodies that share short
names. Because of this, concrete calls skip argument checking (#935), and
the typechecker never produces a resolved callee identity that lowering
could consume (#933).

## Evidence (2026-10-27)

- `src/compiler/typechecker/call_fn.ark` L153–157 states the reason for not
  unifying arguments: "flat name lookup is not overload-aware and would
  false-fail stdlib bodies that share short names."
- `src/compiler/typechecker/call_fn.ark` `tc_strip_module_prefix` and
  `sig_lookup.ark` key signatures by bare name.
- `src/compiler/typechecker/stmt_let.ark` L48–52 documents a second symptom:
  "`Vec::from_iter` picking the first impl by name" produces
  concrete-but-wrong argument types, so annotations are allowed to override
  inferred types (#937).

## Expected

Every call / method call / path call is resolved in the checker to one
declaration identity (module path + item + impl), which carries its exact
signature. Ambiguity is a diagnostic, never "first match by name".

## Acceptance

- [ ] Signature lookup is keyed by module-qualified symbol identity (and impl
      for methods / associated functions); no bare-name fallback.
- [ ] The checker records the resolved FunctionId (and MonoInstanceId for
      generic calls) per call node and exposes it in `TypeCheckResult` for
      #933.
- [ ] Two modules exporting the same function name and calling each other's
      function with different signatures type-check correctly (new fixture
      under `tests/fixtures/modules/`).
- [ ] `Vec::from_iter`-style multiple impls of one associated name are
      selected by type, not declaration order (fixture).
- [ ] `python3 scripts/manager.py selfhost fixpoint` stays green.
