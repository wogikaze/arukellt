---
Status: open
Created: 2026-10-11
Updated: 2026-10-11
ID: 916
Track: compiler-internal
Depends on: none
Orchestration class: investigation
Orchestration upstream: none
Blocks v{N}: language-v2
Priority: 1
Source: #915 fix - unique fresh-var names unmasked deterministic invalid-wasm emission in for-desugared generic bodies
---

# 916 - for-loop desugar falls back to erased generic shells instead of demanding mono for T::into_iter/next

## Summary

A `for value in source` loop inside a generic function body is lowered by
`mir_for_*_callee` resolution that only *finds existing* mono variants and
otherwise falls back to the bare-owner generic shell
(`FilterMapIter::into_iter`, param `structref`, returning an erased ref).
When the concrete impl method was never demanded anywhere else in the
program, no `Type__method__args` mono exists, so the emitted module calls
the erased shell, then the `next` call on the erased iterator type has no
resolution and is emitted `drop; unreachable`, the loop variable falls
back to `i32`, and downstream intrinsic selection (`len` -> `array.len`)
produces a module that `wasm-tools validate` rejects.

## Evidence

- `tests/fixtures/stdlib_trait/iterator_string_callbacks.ark` (~2s post-#915):
  `total_string_lengths__FilterMapIter_String_String_`,
  `__SkipMapIter_VecIter_String__`, `__SkipFilterIter_VecIter_String__`
  all call the generic `$X::into_iter` shell then `unreachable`;
  `$value` is `i32` and `len` emits `array.len`.
- `tests/fixtures/stdlib_trait/iterator_adapters.ark`: same class, func 90
  mismatch.
- Working contrast: every `total_string_lengths__TakeIter_*_` body calls
  mono `$TakeIter__into_iter__X` + `$TakeIter__next__X`. That mono exists
  only because `TakeIter::collect` contains its own `for` loop and
  demanded `TakeIter__into_iter__*` earlier. `FilterMapIter::collect`
  delegates to `collect_iterator` (no `for`), so its `into_iter` mono is
  never minted.
- Compile-time timeouts in the same area (pre-existing, unchanged by
  #915): `stdlib_hashmap/hashmap_generic_demo.ark`,
  `stdlib_hashmap/hashmap_growth.ark`, `stdlib_trait/iterator_hashmap_keys.ark`
  all exceed 180s on both the pre- and post-#915 s2 - probably the same
  inference/mono machinery, confirmed not a #915 regression.

## Why this surfaced now

Under the #915 name collisions these fixtures failed *slowly*: the
pre-#915 s2 did not finish `iterator_string_callbacks.ark` within 10
minutes (mono explosion or loop). Post-#915 the same under-inference
completes fast but emits invalid code. The collision was masking the
resolution gap, not fixing it.

## What a fix looks like

- For-desugar resolution must *demand* the parametric mono
  (`mir_parametric_impl_callee` target) when only the generic shell
  exists - mint `FilterMapIter__into_iter__String__String` and rewrite
  the call, instead of returning the erased shell.
- If resolution still yields only the erased shell for a concrete source
  type, report a compile diagnostic rather than emitting a call into a
  signature that erases `self`/`Item`.
- The emitter must never leave a `for`-desugared `next` call as
  `drop; unreachable` in reachable code: ICE loudly (same stance as the
  #913 guards) so invalid modules cannot ship silently.

## Related

- #915 fresh-var namespace collision (done) - unmasked this gap.
- #851 selfhost compiler core rewrite - the mono/inference internals this
  touches are the rewrite target; durable fix belongs there.
- #912 mono demand-closure boundary - same machinery, minting side.

## Resolution notes (2026-10-11)

Root cause narrowed one level further than the original note: the MIR
resolver was only the *symptom*. The real gap is at check time -
`stmt_for::check_generic_for_source` validates the bound for a
TYPE_VAR `for` source and records nothing, so the desugared
into_iter/next calls had no template site to rebind when the enclosing
generic function was instantiated.

Fix: `record_generic_for_desugar_sites` records var-method template
sites for `into_iter` on T and `next` on the `T::IntoIter` projection
(or `next` on T directly for Iterator-bound sources).
`record_var_method_site` was relaxed to treat `T::Assoc` TY_STRUCT
projections as var-carrying when T is an enclosing type param. The
existing `rebind_enclosing_template_sites` -> `resolve_var_method_site`
-> `mint_trait_method_instance` path then mints
`Owner__method__args` demanded at each concrete instantiation, and the
MIR parametric lookup resolves them exactly like the TakeIter case
that already worked.

Verification:

- `iterator_string_callbacks.ark` + `iterator_adapters.ark`: compile
  ~2-4s AND `wasm-tools validate` clean (previously emitted invalid
  modules with `expected i32, found (ref null $type)`).
- Full fixture sweep: 1786 PASS / 3 FAIL (was 1783/5). All 3 remaining
  failures are the pre-existing compile-timeout class
  (hashmap_generic_demo, hashmap_growth, iterator_hashmap_keys) -
  confirmed by >150s timeouts on the pre-#915 producer s2 as well.
  No new failures and no remaining invalid-wasm emissions.
