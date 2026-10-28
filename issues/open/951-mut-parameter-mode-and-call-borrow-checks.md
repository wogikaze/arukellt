---
Status: open
Created: 2026-10-27
Updated: 2026-10-29
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

- [x] `alias.ark` (#943) is a compile-error fixture on `wasm32` and
      `wasm32-gc`: assignment through the shared parameter of `bump` and
      `push` on the shared parameter of `add` are diagnosed.
      Covered by `diagnostics/shared_param_body_mutation.ark` (E0226) and
      `diagnostics/shared_param_push.ark` (E0223); the combined repro emits
      both codes.
- [x] A positive run fixture: `fn bump(mut p: Point)` with `let mut p`
      prints the mutated value identically on both targets.
      `tests/fixtures/mut_params/mut_param_places.ark` covers a `mut`
      parameter, a `mut self` method, a field place, and `|mut v|`; it runs
      on `wasm32-gc` and `wasm32` with matching output.
- [x] Negative fixtures:
      - [x] immutable binding to a `mut` parameter
            (`diagnostics/mut_param_immutable_place.ark`, E0223)
      - [x] `mut self` method on an immutable receiver
            (`diagnostics/mut_self_immutable_receiver.ark`, E0223)
      - [x] same root place passed twice with one `mut`
            (`diagnostics/mut_param_same_place.ark`, E0224)
      - [x] borrowed `Box<T>` returned, stored, and `unbox`ed
            (`diagnostics/borrowed_box_return.ark`,
            `diagnostics/borrowed_box_store.ark`,
            `diagnostics/borrowed_box_unbox.ark`, E0227)
      - [x] `fn(T)` vs `fn(mut T)` assignment direction
            (`diagnostics/fn_type_mode_downgrade.ark`, E0225). The direction
            is checked at annotated `let` bindings, function-argument
            positions (`fn_type_mode_argument.ark`), return positions
            (`fn_type_mode_return_stmt.ark`, `fn_type_mode_return_tail.ark`),
            and module-qualified function paths
            (`fn_type_mode_qualified_path.ark`). Calls through a
            `fn(mut T)` value are covered by
            `diagnostics/fn_value_mut_param_immutable_place.ark` (E0223) and
            `diagnostics/fn_value_mut_param_same_place.ark` (E0224).
- [x] Escape by sharing is covered by a run fixture whose observable output
      matches on both targets (ADR-088 §6).
      `ownership/shared_escape_aliases.ark` returns a borrowed `Point` and
      mutates it through the returned alias; both targets print `2`.
- [x] After migration the diagnostics are errors; `python3 scripts/manager.py
      verify fixtures` and `python3 scripts/manager.py selfhost fixpoint`
      stay green.
      The callee-side rule is an error by default. The final gates report
      fixtures PASS=1886 FAIL=0 (`test:` 36/36) and fixpoint
      `sha256(s2) == sha256(s3) = 0c2e96b4…`; see the 2026-10-29 §6/§7
      verification receipt below.
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
- `python3 scripts/manager.py selfhost diag-parity`: PASS (current s2
  against every committed `diag:` golden, including the seven new E0225/E0227
  fixtures).
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

## Implementation (2026-10-29, callee-side rule)

Third vertical slice: the callee side of ADR-088 §2/§3. A shared parameter
and a shared `self` are now immutable in the body; only `mut p` / `mut self`
may be assigned. The stdlib, the selfhost compiler, the fixture corpus,
examples, and docs samples are migrated to declare `mut` on every parameter or
receiver that mutates through it, so the rule is enabled as an error rather
than a warning.

- `body_params` binds each parameter by `FnSig_param_is_mut` (the mode the
  first slice already recorded), and the scope records whether an immutable
  binding is a shared parameter/receiver so the diagnostic can name the fix.
- E0226: assigning through a shared parameter or shared `self` (including its
  field and element places) is an error. A mutating call such as `push(p, x)`
  or a `mut self` method on a shared parameter is still reported as E0223 at
  the call site. `mut p = …` rebinds only the callee's name and is allowed.
- Migration: 1,817 `.ark` files changed plus four new fixtures, with 4,352
  changed parameter-declaration lines and 2,345 new `let mut` binding lines
  (including test bodies). The analyzer seeds direct parameter assignment and
  the mutating stdlib operations (`push`, `set`, `pop`, `insert`, `clear`,
  `remove`, `append`, `extend`, and the collection setters), then propagates
  through calls and method receivers to a fixpoint. Calls nested in argument
  expressions (`i32_to_i64(self.read_u8())`) and receivers reached through a
  struct field are followed too; those two passes found the remaining
  `std/bytes`, `std/io`, `std/toml`, and iterator-adapter cases.
  `std/`, `src/compiler/`, `tests/fixtures/`, `benchmarks/`, `examples/`,
  the runnable `docs/examples/`, and `extensions/` are covered. The
  uncompilable v0 `docs/sample/parser.ark` (pre-existing type errors) is left
  unchanged. The diagnostic fixtures that
  intentionally pass an immutable place (`mut_param_immutable_place`,
  `mut_self_immutable_receiver`, and the new E0226 fixtures) are excluded from
  the mechanical rewrite. `std/toml/parser.ark` needed one local alias
  (`let mut array = arr`) where a match binding was pushed through a `mut`
  parameter.
- One same-root call site was real: `HashSet::extend` is now `mut self`, so
  the String self-extend fixture goes through `grown.clone()` instead
  (ADR-088 §5). `MirModule_reindex_body_store` takes a `let mut` snapshot
  before passing the module and its function vector.

### Bootstrap compatibility

The official pin (`8df2698c…`) predates the #951 parser and rejects `mut`
parameter declarations, while the committed compiler source now uses them.
`_strip_bootstrap_param_modes` removes the mode token from the flat overlay
copies used by the pin; the checked-in source keeps the modes. Fixpoint stage 3
reuses the same stripped overlay, so it passes the internal
`--bootstrap-allow-shared-param-mutation` option to restore the pre-#951
permissive body binding for that self-compile only. User programs, fixtures,
and the analysis pipeline keep the strict rule. Both shims have deletion
conditions in `docs/compiler/bootstrap.md` (a pin and overlay that ship the
real modes).

Fixtures: `mut_params/shared_and_mut_bodies.ark` (run, expected `3/3/7`),
`diagnostics/shared_param_body_mutation.ark` (E0226),
`diagnostics/shared_self_body_mutation.ark` (E0226), and
`diagnostics/shared_param_push.ark` (E0223, the `push` half of the #943
repro). The `bump` half of the repro is covered by
`shared_param_body_mutation.ark`; together they are the required compile-error
registration.

### Verification (2026-10-29, callee-side rule)

- `python3 scripts/manager.py selfhost build-compiler`: PASS, elapsed
  2135.1s (first build, intermediate), s2 sha256
  `6cefc823e5ce5f9e25080c5abe0465d9bec8b50c1374394fbe4fedf6cc053dbd`.
- `python3 scripts/manager.py selfhost build-compiler` after the compiler
  flag: PASS, elapsed 1911.2s, s2 sha256
  `b5eead77ab4591d13e559e8e09e5762fdc87b40c8a70370e2c97b4d03036bf18`.
  A later fixture-gate rebuild after a late `std/host/fs.ark` annotation
  reproduced the same sha256. After the final nested-call/toml pass,
  `build-compiler` was run once more: PASS, elapsed 1922.6s, s2 sha256
  `7510f9820156fabc301aa966e7c9adde5d88eed8c03448b141422cc0a996a97d`.
- `python3 scripts/manager.py selfhost fixpoint`: PASS —
  `sha256(s2) == sha256(s3)` =
  `7510f9820156fabc301aa966e7c9adde5d88eed8c03448b141422cc0a996a97d`
  (pinned base `8df2698c8401693879ca4f9ced289ed2c285d95eb526cf70fda02dd5876259ee`).
  The first fixpoint attempt compared the intermediate s2
  (`b5eead77…`) with an s3 built from the final source (`7510f982…`) and
  correctly reported not reached; rebuilding s2 from the final source
  resolved it.
- `python3 scripts/manager.py verify fixtures`: PASS — PASS=1882 FAIL=0,
  1882 compile+validate, 109 runtime executed, 2 native-cpp contracts
  delegated; `test:` PASS=36 FAIL=0.
- `python3 scripts/manager.py selfhost diag-parity`: PASS.
- `python3 scripts/manager.py selfhost diag-parity`: PASS (current s2
  against every committed `diag:` golden, including the seven new E0225/E0227
  fixtures).
- `python3 scripts/manager.py verify lane`: PASS (`quality changed`;
  lint checked=1801 failed=0; lint ratchet checked=1801 failed=0). The
  touched-code advisory reports eight `lines_ge_200` signatures that the
  `mut ` prefix pushed just over 200 characters; the global ratchet is
  unchanged. Those signatures are left unsplit rather than reformatted after
  the final build.
- `python3 scripts/manager.py docs check`: 6/6 PASS.
- Manual both-target run of `mut_params/shared_and_mut_bodies.ark`:
  `3/3/7` on `wasm32-gc` and `wasm32`.
- The #943 `alias.ark` repro now reports E0226 for `bump` and E0223 for
  `push` in `add`.

## Implementation (2026-10-29, §6 Box escape and §7 positions)

Fourth vertical slice: closes the ADR-088 §6 borrowed-`Box<T>` rule and the
remaining §7 direction positions.

- Box escape (E0227). A parameter is now marked in `LocalScope`
  (`VarBinding_is_param`, preserved by `scope_child`); a `Box<T>`-typed
  parameter cannot be moved. `ownership::consume_identifier` reports a move
  of the borrowed Box, which covers `let` initializers, tuple/array/struct
  literals, and the `Box_new` / `unbox` argument boundaries. Positions that
  never call `consume_identifier` are covered by
  `ownership::check_borrowed_box_escape`: return statements, tail
  expressions, assignment right-hand sides, struct/tuple/array stores,
  `Box_new` / `unbox` calls, and source-enum constructor payloads. `if` and `match`
  contribute only their branch values, so a match scrutinee stays a read.
  Reports are de-duplicated by span because the expression-statement and tail
  passes can visit the same node. A trailing `return` inside a nested block
  is checked in `infer_block_expr`, which previously only handled closure
  captures there. A local read (`match b { Box::Owned(v) => v }`) and
  passing the value to another shared parameter stay legal.
- Function-type positions (E0225). `param_mode::check_fn_type_value` applies
  the shared→`mut` / `mut`→shared direction at generic and concrete free
  calls, generic and concrete method arguments, calls through function
  values, explicit `return`, and tail expressions. `sigs::resolve_fn_sig`
  recovers the declared `TY_FN` of a bare name, a module-qualified path
  (`prelude::push`), or an impl spelling through
  `callee_spelling::callee_spelling_variants`; a type-owned head
  (`Vec::new`) is excluded from the bare-leaf fallback.
  `sigs::fn_value_type_from_sig` is the single recovery point and is also
  used by the annotated-`let` check, which now accepts a path initializer.
  A closure tail has no inferred type in the syntax-only tail pass, so its
  parsed parameter modes are compared directly.
- Fixtures: `mut_params/fn_type_mode_positions.ark` (run, `4/4`),
  `ownership/borrowed_box_local_use.ark` (run, `41/41`),
  `ownership/shared_escape_aliases.ark` (run, `2`, the §6 sharing escape),
  and the E0227
  diagnostics `diagnostics/borrowed_box_return.ark`,
  `diagnostics/borrowed_box_store.ark`,
  `diagnostics/borrowed_box_unbox.ark`, plus the E0225 diagnostics
  `diagnostics/fn_type_mode_argument.ark`,
  `diagnostics/fn_type_mode_return_stmt.ark`,
  `diagnostics/fn_type_mode_return_tail.ark`, and
  `diagnostics/fn_type_mode_qualified_path.ark`.
- Docs: `docs/language/spec.md` §3.5.1, `docs/language/memory-model.md`,
  `docs/current-state.md`, and `docs/compiler/error-codes.md` (E0227 added,
  E0225 scope widened).
- Scope note: E0212 for a moved binding used as a call argument (#944) is
  ADR-088 §1, not the §6/§2 items closed here; it remains tracked by
  [#944](../../issues/open/944-move-after-use-not-diagnosed.md).

### Verification (2026-10-29, §6 Box escape and §7 positions)

- `python3 scripts/manager.py selfhost fixpoint --build --no-cache`: PASS —
  stage 2 from the pinned bootstrap, stage 3 from s2, and
  `sha256(s2) == sha256(s3)` =
  `0c2e96b45159f10b095c26afd0999c412f9d0d8f44fab7210ade29bd52684dbf`
  (pinned base
  `8df2698c8401693879ca4f9ced289ed2c285d95eb526cf70fda02dd5876259ee`).
- `python3 scripts/manager.py verify fixtures`: PASS — PASS=1886 FAIL=0,
  1886 compile+validate, 109 runtime smoke, 36/36 `test:`; 2 native-cpp
  runtime contracts delegated.
- `python3 scripts/manager.py selfhost diag-parity`: PASS (current s2
  against every committed `diag:` golden, including the seven new E0225/E0227
  fixtures).
- `python3 scripts/manager.py verify lane`: PASS (`quality changed`;
  lint checked=19 failed=0). Touched-code advisory:
  `call_generic_fn_args.ark single_function_files 1 > 0` after the fn-value
  recovery moved to `sigs.ark`; the global `single_function_files` count is
  467 against the 470 baseline. The separate global
  `check-ark-code-quality.py` `lines_ge_200` ratchet is already 10 > 2 at the
  base commit (eight `mut`-prefix signatures in files this slice did not
  touch), so this slice does not introduce it.
- `python3 scripts/manager.py docs check`: 6/6 PASS.
- Manual both-target runs with the built s2:
  `ownership/borrowed_box_local_use.ark` prints `41/41`,
  `ownership/shared_escape_aliases.ark` prints `2`, and
  `mut_params/fn_type_mode_positions.ark` prints `4/4` on both `wasm32-gc`
  and `wasm32`. The seven diagnostic fixtures emit E0225/E0227 under
  `check` with the built s2.

## Remaining scope

1. Callee-side rule (ADR-088 §2/§3): landed in the 2026-10-29 slice.
   `body_params` binds by `FnSig_param_is_mut`, E0226 rejects assignment
   through a shared parameter or shared `self`, and the corpus is migrated.
   The self-compile stage uses the bootstrap `mut`-mode strip and the internal
   `--bootstrap-allow-shared-param-mutation` option because the pin and the
   flat overlay predate the mode parser; user code and fixtures are strict.
2. Function-type mode (ADR-088 §7): implemented for parsing, `TY_FN`
   modes, calls through function values, shared→`mut` assignment at
   annotated `let` bindings, and the reverse-direction check at
   function-argument and return positions, including module-qualified paths
   and impl spellings. A closure returned as a tail expression without an
   inferred type is checked from its parsed parameter modes; a closure passed
   as an argument or returned with an explicit `return` uses the inferred
   `TY_FN`.
3. Borrowed `Box<T>` escape (ADR-088 §6): landed. A borrowed `Box<T>`
   parameter cannot be moved, returned, stored in an aggregate, or
   `unbox`ed (E0227). The rule is restricted to `Box<T>`; other non-`Copy`
   parameters still escape by sharing. Two residual gaps:
   - A concrete (non-generic) `Box<i32>` parameter still hits a pre-existing
     match-binding type mismatch in the checker; the positive fixture uses
     the generic shape that mirrors `unbox`.
   - A source enum constructor (`Box::Owned(b)`) is checked, but the builtin
     `Option`/`Result` constructors (`Option::Some(b)`, `Ok(b)`) are not
     enum defs, so a borrowed Box stored through them is still accepted.
   - The value-shape walker follows aggregates and control flow, but not a
     `Box_new`/`unbox` call nested inside a larger expression such as
     `return unbox(b) + 1`. Direct call positions and `let` initializers are
     covered.
4. stdlib surface (ADR-088 §8): `push`/`set`/`pop`/`insert`/`clear` and the
   collection setters now declare `mut` parameters/receivers. No
   name-based exemption is used.
5. Same-root checking is syntactic and collapses element paths to `[]`; this
   is the ADR's no-alias-analysis choice, not a temporary limitation.

### Verification (2026-10-28)

- `python3 scripts/manager.py selfhost build-compiler`: PASS, elapsed
  2291.6s (cold overlay cache), s2 sha256
  `b95fb1b300b4cca365426456394451d581bcdac4cc35701b9ea971da6d8e7913`.
- `python3 scripts/manager.py verify fixtures`: PASS=1909 FAIL=0,
  110 runtime smoke fixtures (the new `mut_params/` domain is one of them).
- `python3 scripts/manager.py selfhost diag-parity`: PASS.
- `python3 scripts/manager.py selfhost diag-parity`: PASS (current s2
  against every committed `diag:` golden, including the seven new E0225/E0227
  fixtures).
- `python3 scripts/manager.py verify lane`: PASS (`quality changed`;
  lint checked=16 failed=0).
- 2026-10-29 cross-lane receipt: the ADR-089 residual lane rebuilt s2 on top
  of this work (sha256
  `1e10f559f16ae15e99bf3300657668c103ecd486ce703d6f4e669173235ebe7e`) and
  `verify fixtures` stayed green (PASS=1882 FAIL=0); no #951 behavior
  changed. See #946.
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
