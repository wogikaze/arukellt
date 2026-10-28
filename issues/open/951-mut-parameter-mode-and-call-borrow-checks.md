---
Status: open
Created: 2026-10-27
Updated: 2026-10-28
ID: 951
Track: typecheck
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "ADR-088 decision for #943 (2026-10-27)"
Related: "ADR-059, ADR-060, ADR-061, ADR-088, ADR-014, #936, #943, #944"
---

# 951 — implement ADR-088: `mut` parameter mode, mutable-place and same-place conflict checks

## Summary

ADR-088 decides that non-`Copy` arguments are call-scoped borrows and that
mutation across a call requires a `mut` parameter (or `mut self`) and a
mutable place at the call site. None of this is implemented: there is no
`mut` parameter syntax (`rg 'fn …\(mut x:' std src/compiler` finds 0 uses),
and today a callee can mutate an immutable caller binding through any
parameter (#943 `alias.ark` repro, observed with `wasmtime run` on both
targets printing `3` / `2`).

## Scope

1. Parser / AST: `mut` before a parameter name, `mut self`, and `|mut v|`
   closure parameters. Formatter and LSP follow (`src/compiler/parser/`,
   `src/compiler/fmt/`).
2. Typechecker, keyed on binding state (`src/compiler/typechecker/ownership.ark`,
   `call_fn.ark`, `call_method.ark`):
   - Assignment to a place rooted at a shared parameter / `self` is
     rejected (E0207 family).
   - Passing a shared-rooted or immutable-binding-rooted place to a `mut`
     parameter, or calling a `mut self` method on it, is rejected with a new
     diagnostic code.
   - The same root place passed to a `mut` parameter and another parameter
     in one call is rejected.
   - A borrowed `Box<T>` parameter cannot be returned, stored, or `unbox`ed
     (ADR-088 §6).
   - The parameter mode is part of `FnSig` and function types; a
     shared-parameter fn value is assignable to the `mut` form only.
3. stdlib: mutating operations (`push`, `set`, `pop`, `insert`, `clear`, …)
   declare `mut` receivers / parameters through their real signatures (#936),
   with no name-based exemption.
4. Migration (ADR-014): the new diagnostics start as warnings with a
   ratchet count, `std/` and `src/compiler/` are migrated (`let mut`, `mut`
   parameters), then the diagnostics become errors.
5. Docs: `docs/language/memory-model.md`, `docs/language/spec.md` §2.4 and
   `docs/current-state.md` describe ADR-088 and the current gap (#943
   acceptance).

## Acceptance

- [ ] `alias.ark` (#943) is a compile-error fixture on `wasm32` and
      `wasm32-gc`: assignment through the shared parameter of `bump` and
      `push` on the shared parameter of `add` are diagnosed.
      Blocked on the callee-side rule: shared parameters are still bound
      mutable, so the alias program still compiles (see Remaining scope).
- [x] A positive run fixture: `fn bump(mut p: Point)` with `let mut p`
      prints the mutated value identically on both targets.
      `tests/fixtures/mut_params/mut_param_places.ark` covers a `mut`
      parameter, a `mut self` method, a field place, and `|mut v|`; it runs
      on `wasm32-gc` and `wasm32` with matching output.
- [ ] Negative fixtures:
      - [x] immutable binding to a `mut` parameter
            (`diagnostics/mut_param_immutable_place.ark`, E0223)
      - [x] `mut self` method on an immutable receiver
            (`diagnostics/mut_self_immutable_receiver.ark`, E0223)
      - [x] same root place passed twice with one `mut`
            (`diagnostics/mut_param_same_place.ark`, E0224)
      - [ ] borrowed `Box<T>` returned (not implemented)
      - [x] `fn(T)` vs `fn(mut T)` assignment direction
            (`diagnostics/fn_type_mode_downgrade.ark`, E0225). The direction
            is checked at annotated `let` bindings; function-argument and
            return positions are not yet checked. Calls through a
            `fn(mut T)` value are covered by
            `diagnostics/fn_value_mut_param_immutable_place.ark` (E0223) and
            `diagnostics/fn_value_mut_param_same_place.ark` (E0224).
- [ ] Escape by sharing is covered by a run fixture whose observable output
      matches on both targets (ADR-088 §6).
      Not added; sharing itself is unchanged, so the fixture would pass
      without exercising a new rule.
- [ ] After migration the diagnostics are errors; `python3 scripts/manager.py
      verify fixtures` and `python3 scripts/manager.py selfhost fixpoint`
      stay green.
      The new diagnostics are already errors, but the migration that makes
      them cover shared parameters has not run.
- [x] Language docs and current-state are synced (`$docs-sync`) for the
      implemented slice; the remaining scope is recorded below and in
      `docs/current-state.md`.

## Implementation (2026-10-28)

Partial vertical slice: syntax plus the call-site half of ADR-088. The
callee-side rule is not enabled because it requires the std/selfhost
migration; the function-type half landed in the second slice below.

- Parser / AST: `mut` before a regular function parameter, a clause
  parameter, `self` (`mut self`), and a closure parameter (`|mut v|`). The
  `mut` keyword is consumed by `parser::expect::consume_optional_mut` and
  recorded in the existing `AstNode.is_mut` flag. The formatter is
  token/line based and needs no change; `fmt/mut_parameters.ark` pins its
  idempotence. LSP symbol details do not render parameter lists, so no LSP
  edit was needed.
- Typechecker: `FnSig` gains `param_modes: Vec<bool>` with
  `FnSig_new_with_param_modes` and `FnSig_param_is_mut`;
  `with_name`/`with_types` preserve the modes. `sig_decl` fills the modes
  from the parameter AST. `typechecker/param_mode.ark` checks resolved
  calls: a non-`Copy` `mut` parameter needs a mutable place (E0223), and one
  call must not pass overlapping root places to a `mut` parameter and
  another parameter (E0224). The receiver position uses the inferred receiver
  type and is checked for `mut self`; `Copy` parameters are exempt. Checks
  run for concrete and generic free calls and for resolved method calls.
  Closure parameters are bound with the parsed mode, so `|mut v|` allows
  assignment in the body.
- Diagnostics: E0223 (`mut` parameter requires a mutable place) and E0224
  (overlapping places). Both are errors. The ADR-014 warning window was not
  run because no pre-existing declaration uses the new spelling; this follows
  the #946 posture.
- Fixtures: `mut_params/mut_param_places.ark` (run, expected `5/11/6/5`),
  `diagnostics/mut_param_immutable_place.ark` (E0223),
  `diagnostics/mut_self_immutable_receiver.ark` (E0223),
  `diagnostics/mut_param_same_place.ark` (E0224), and
  `fmt/mut_parameters.ark` (formatter idempotence).

## Implementation (2026-10-28, function-type modes)

Second vertical slice: the parameter mode is now part of a function type,
covering the self-contained half of ADR-088 §7. This slice adds no `mut`
spelling to `std/` or `src/compiler/`, so it does not need the callee-side
migration.

- Parser / AST: `fn(mut T) -> U` is parsed by
  `parser::types::parse_fn_type_param`, which consumes the optional `mut` and
  records it on the parameter type node (`is_mut`). The tuple-form function
  type `(T) -> U` keeps its existing syntax; only the `fn(...)` form accepts
  `mut`.
- `TypeInfo` (`corehir::type_contracts` and the typechecker mirror used by the
  flat selfhost overlay) gains `param_modes: Vec<bool>`, aligned with the
  parameter prefix of `type_args`; an empty vector is the shared default.
  `TypeInfo_empty_like` preserves the modes when instantiation or deep
  resolution rebuilds a TY_FN.
- `TY_FN` construction fills the modes from explicit annotations
  (`type_ann`, `type_generic`), closure parameters (`infer_closure`), and
  named function values (`call_generic_fn_args::fn_value_type_from_sig`).
- Calls through a function value (`call_unknown::infer_local_function_call`)
  run the same E0223/E0224 checks as direct calls, using the modes stored in
  the type.
- Assignment direction at annotated `let` bindings: a shared-parameter
  function value is assignable to a `mut` position, and the reverse is E0225
  (`DIAG_TYPE_FN_MODE_ASSIGN`) in `param_mode::check_fn_type_assignment`.
  `stmt_let` recovers a bare top-level function's TY_FN from its `FnSig` so
  `let f: fn(Point) -> i32 = bump` can be checked.
- Docs/error catalogue: E0225 is declared in
  `src/compiler/diagnostics/codes.ark` and documented in
  `docs/compiler/error-codes.md`; `docs/current-state.md`,
  `docs/language/spec.md` §3.5.1, and `docs/language/memory-model.md` record
  the implemented slice and its residual.

Fixtures: `mut_params/fn_type_modes.ark` (run, expected `1/1/6/5/2/2`),
`diagnostics/fn_type_mode_downgrade.ark` (E0225),
`diagnostics/fn_value_mut_param_immutable_place.ark` (E0223),
`diagnostics/fn_value_mut_param_same_place.ark` (E0224), and
`fmt/fn_type_mut.ark` (formatter idempotence).

### Verification (2026-10-28, function-type modes)

- `python3 scripts/manager.py selfhost build-compiler`: PASS, elapsed
  1918.4s, s2 sha256
  `b9117f65b0b22e47fd419b816927f55e1c0d139049263bc812364b1e58e42bf0`.
- `python3 scripts/manager.py verify fixtures`: PASS — PASS=1914 FAIL=0,
  1914 compile+validate, 110 runtime smoke, 2 native-cpp runtime contracts
  delegated.
- `python3 scripts/manager.py selfhost diag-parity`: PASS.
- `python3 scripts/manager.py selfhost fmt-parity`: PASS=6 FAIL=3. The
  three failures (`fmt/delimiters_in_strings.ark`,
  `fmt/imports_interleaved.ark`, `fmt/imports_unsorted.ark`) reproduce with
  the pinned bootstrap compiler, so they predate this slice;
  `fmt/fn_type_mut.ark` passes.
- `python3 scripts/manager.py verify lane`: PASS (`quality changed`;
  lint checked=17 failed=0). The touched-code thin-wrapper advisory lists
  the three contract accessor files; the global ratchet is unchanged.
- `python3 scripts/manager.py docs check`: 6/6 PASS.
- Manual both-target run of `mut_params/fn_type_modes.ark`: the wasm32-gc
  component and the wasm32 core module both print `1/1/6/5/2/2`, matching
  `.expected`.
- Current s2 emits the committed `.diag` lines for the three new diagnostic
  fixtures (E0223/E0224/E0225).
- `python3 scripts/manager.py selfhost fixpoint`: PASS — stage 3 was
  rebuilt from s2 and `sha256(s2) == sha256(s3)` =
  `b9117f65b0b22e47fd419b816927f55e1c0d139049263bc812364b1e58e42bf0`
  (pinned base sha256
  `8df2698c8401693879ca4f9ced289ed2c285d95eb526cf70fda02dd5876259ee`).

## Remaining scope

1. Callee-side rule (ADR-088 §2/§3): `body_params` still binds every
   parameter mutable, so assigning through a shared parameter or shared
   `self` is accepted. Because the same binding state backs the call-site
   check, a shared parameter or shared `self` is also accepted where a
   mutable place is required. Enabling the rule requires migrating `std/`
   and `src/compiler/` mutating parameters to `mut` and their call sites to
   `let mut`; `fn bump(p: Point) { p.x = ... }` and the #943 `alias.ark`
   repro therefore still compile (verified: prints `3`/`2`).
2. Function-type mode (ADR-088 §7): implemented for parsing, `TY_FN`
   modes, calls through function values, and shared→`mut` assignment at
   annotated `let` bindings. Remaining: the reverse-direction check is not
   applied when a function value is passed as an argument or returned, and
   qualified/path function names are not recovered for the `let` check.
3. Borrowed `Box<T>` escape (ADR-088 §6): returning, storing, or `unbox`ing a
   borrowed `Box<T>` parameter is still accepted.
4. stdlib surface (ADR-088 §8): `push`/`set`/`pop`/`insert`/`clear` are not
   declared with `mut` receivers yet.
5. Same-root checking is syntactic and collapses element paths to `[]`; this
   is the ADR's no-alias-analysis choice, not a temporary limitation.

### Verification (2026-10-28)

- `python3 scripts/manager.py selfhost build-compiler`: PASS, elapsed
  2291.6s (cold overlay cache), s2 sha256
  `b95fb1b300b4cca365426456394451d581bcdac4cc35701b9ea971da6d8e7913`.
- `python3 scripts/manager.py verify fixtures`: PASS=1909 FAIL=0,
  110 runtime smoke fixtures (the new `mut_params/` domain is one of them).
- `python3 scripts/manager.py selfhost diag-parity`: PASS.
- `python3 scripts/manager.py verify lane`: PASS (`quality changed`;
  lint checked=16 failed=0).
- `python3 scripts/manager.py docs check`: 6/6 PASS.
- `python3 scripts/manager.py selfhost fmt-parity`: PASS=5 FAIL=3. The three
  failures (`fmt/delimiters_in_strings.ark`, `fmt/imports_interleaved.ark`,
  `fmt/imports_unsorted.ark`) reproduce with the pinned bootstrap compiler, so
  they predate this change; `fmt/mut_parameters.ark` passes.
- Manual both-target run of `tests/fixtures/mut_params/mut_param_places.ark`:
  `wasm32-gc` component prints `5/11/6/5`; `wasm32` core module prints
  `5/11/6/5`. The three diagnostic fixtures emit the committed `.diag`
  strings on both targets.
- `python3 scripts/manager.py selfhost fixpoint`: PASS — stage 3 was
  rebuilt from s2 and `sha256(s2) == sha256(s3)` =
  `b95fb1b300b4cca365426456394451d581bcdac4cc35701b9ea971da6d8e7913`.
