---
Status: open
Created: 2026-10-11
Updated: 2026-10-11
ID: 915
Track: compiler-internal
Depends on: none
Orchestration class: investigation
Orchestration upstream: none
Blocks v{N}: language-v2
Priority: 2
Source: #815 unskip work - ambiguous-param diagnostic fired on format_debug calls with concrete args
---

# 915 - Fresh type-var names collide across TypeEnv scopes

## Summary

Every TypeEnv numbers fresh inference variables from the same namespace:
`fresh_var` names vars `t<env.next_id>` and each per-call infer env
starts at `t0`. The TYPE_VAR same-name early return in
`unify_core.ark` treats equal names as the same declared parameter, so
a fresh var `t0` minted in the main env (e.g. by a generic-call result
or a variant-payload binding) and a fresh var `t0` minted in a generic
call's scratch infer env silently compare as equal. The same-name
shortcut then returns without binding, so the call's variable is left
unconstrained by an argument it should have unified with.

All var-keyed operations (`bind_var`, `resolve_type`,
`occurs_in_type`, `is_unresolved_fresh_type_var`) look TypeVars up by
name only - TypeInfo has no identity/id - so the two distinct vars are
indistinguishable throughout unification.

## Evidence

- `use std::text::fmt` plus a body call `fmt::format_debug(val)` where
  `val` is bound by `Option::Some(val)` in a match arm: the call's
  `T` stays unbound. Under the #815 ambiguous-param diagnostic this
  produced `cannot infer type parameter T in call format_debug` - 34
  spurious diagnostics inside `std/text/fmt.ark` alone - while the
  identical call on a literal (`format_debug(1)`) or a fn parameter
  infers correctly.
- The same collision likely under-infers other generic calls whose
  argument type is a fresh var from the enclosing env; previously it was
  invisible because unresolved calls simply skip mono minting.

## What a fix looks like

Options (design choice belongs with the #912 mono/inference work):

- Give each TypeEnv scope a unique fresh-var namespace (e.g. seed
  `next_id` from a shared counter or a per-scope prefix) so
  cross-scope name equality is impossible.
- Or carry real identity on TypeInfo/TypeVar and make the same-name
  shortcut, bind, resolve, and occurs checks compare identity, not
  strings.

The narrow #815 diagnostic is deliberately scoped to provably-ambiguous
zero-arg calls to avoid depending on this fix.

## Related

- #912 monomorphization demand-closure - inference/mono internals share
  the same flat name-keyed tables; namespace cleanup belongs together.
- #914 generic name-collision fix (done) - the sibling bug at the
  fn-signature name-index level.

## Resolution notes (2026-10-11)

Implemented the shared-counter option: `TypeEnv.next_id` became
`var_id_box: Vec<i32>`, a one-element box shared by every env derived
from a parent (`borrow_generic_infer_env`, `type_env_for_fn_check`), so
`t<id>` names are globally unique within a check session and the
unify same-name shortcut can never alias cross-scope vars.

Verification:

- `tests/fixtures/generics/`: 8/8 FAIL -> 8/8 PASS.
- `check std/text/fmt.ark`: zero `cannot infer type parameter`
  diagnostics (only pre-existing W0005 export warnings).
- `tests/fixtures/stdlib_trait/format_debug_match_binding.ark` added as
  scenario coverage (match-bound `val` into `fmt::format_debug`).
- Full fixture sweep: 1783 PASS / 5 FAIL; all 5 verified pre-existing
  (see #916 - three hashmap/iterator timeouts reproduce on the pre-#915
  s2, two iterator validation errors are a mono-minting gap the
  collision was masking).

Latent issue fixed along the way: leaked scratch vars (now correctly
distinct) reached `enforce_operator_trait` bound checks as
non-declared vars and produced spurious E0208s. Bound checks now
apply to declared `current_fn_type_params` only; fresh vars defer to
unification.
