---
Status: done
Created: 2026-10-27
Updated: 2026-10-29
ID: 944
Track: typecheck
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-059, ADR-088, E0212, #943, #945, #951"
---

# 944 — use of a moved binding is not diagnosed when the use is a call argument

## Summary

ADR-059 §4: reuse of a moved non-`Copy` binding is E0212. Move tracking only
covers particular AST shapes. `consume_expr` returns early for method calls,
paths, blocks, binaries, and so on, and for plain calls it only consumes the
`Box_new` / `unbox` boundary. Identifier inference marks the binding as used
but does not check its moved state. After `let b = a`, passing `a` to any
function is accepted, and `a` and `b` alias.

## Evidence (2026-10-27)

- `src/compiler/typechecker/ownership.ark` L46–96 (`consume_expr`; `NK_CALL`
  → `consume_box_boundary_argument` only).
- `src/compiler/typechecker/infer_leaf.ark` L40–50 (identifier inference
  does not consult moved state).

Repro 1 (`ext_move.ark`):

```ark
use std::host::stdio

struct P {
    s: String,
}

fn show(p: P) {
    stdio::println(p.s)
}

fn main() {
    let a = P { s: String_from("a") }
    let b = a
    show(a)
    show(b)
}
```

Repro 2 (`move_let.ark`):

```ark
use std::host::stdio

fn main() {
    let v1 = Vec::new<i32>()
    let v2 = v1
    push(v1, 1)
    stdio::println(len(v2))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`): both compile on `wasm32` and `wasm32-gc`;
`wasmtime run` prints `a`, `a` and `1`. No E0212 is reported.

## Expected

ADR-059 §4 and ADR-088 §1 (decided 2026-10-27 for #943): passing a moved
binding as an argument is E0212, like any other use. Both repros are E0212
at the use of the moved binding (`show(a)`, `push(v1, 1)`). Passing a
non-moved binding is a call-scoped borrow and does not move it.

## Acceptance

- [x] Binding state (moved / Copy / borrowed) is checked at every use of an
      identifier in one place, not per AST shape. Expression reads share
      `ownership::check_moved_binding_use`; call-position uses share
      `ownership::is_moved_binding`; the non-call reads below are inside the
      slice.
- [x] Both repros are registered compile-error fixtures (E0212) on both
      targets.
- [x] Existing ownership / closure-capture fixtures (E0212–E0214) stay
      green.
- [x] `python3 scripts/manager.py selfhost fixpoint` stays green (selfhost
      sources migrated if the rule tightens).

## Implementation (2026-10-29, call arguments and receivers)

The call-boundary half of ADR-088 §1 is implemented as a single place check.

- `ownership::check_call_place_uses` runs from `infer_compound_expr` for every
  `NK_CALL` (arguments from child 1) and every `NK_METHOD_CALL` (the receiver
  at child 0 plus the arguments from child 1), before callee resolution. Free
  calls, method calls, calls through function values, builtins, and unresolved
  calls therefore share the same rule.
- `ownership::check_moved_place_use` follows a `NK_FIELD_ACCESS` or `NK_INDEX`
  argument / receiver to its root binding and reports E0212 through the
  existing `move_after_use_message` hint. A live argument is not marked moved,
  so repeated call-scoped borrows stay legal.
- `ownership::is_moved_binding` is the single moved / `Copy` predicate for the
  call-position use. `consume_identifier` now reports through
  `report_ownership_error_once`, so a `Box_new` / `unbox` boundary that both
  the call check and the move path visit reports one E0212 per span. The E0227
  escape walker reuses the same de-duplication helper.
- `param_mode::check_mutable_parameter_places` skips a moved place: the
  argument already carries E0212, so the `mut`-place rule does not add a
  second diagnostic for the same argument.
- Selfhost migration: four real move-then-call-argument sites in the compiler
  source used a moved binding again (`infer_closure`'s `body_ty`,
  `post_pass_type_propagate` and `post_pass_callee_lookup`'s `dst_loc`, and
  `gc_layout_table`'s `key`); they now clone the value where the original is
  still read. The first stage-3 fixpoint run found the `gc_layout_table` case,
  which the initial line-based audit missed because it is a multi-line struct
  literal; the second `build-compiler` run carries that fix.
- Fixtures: `diagnostics/moved_argument_free_call.ark` (repro 1),
  `diagnostics/moved_argument_push.ark` (repro 2, E0212 without E0223),
  `diagnostics/moved_argument_method_receiver.ark`,
  `diagnostics/moved_argument_function_value.ark`, and
  `diagnostics/moved_argument_field_place.ark` are `diag:` entries;
  `ownership/call_borrow_new_owner.ark` (`.expected` `5/5/5`) is a positive
  `run:` entry that proves a live argument is borrowed rather than moved.
  `diagnostics/mutable_sharing.ark` reused a binding after moving it into an
  alias, so its post-move `push` is now E0212 and suppresses the W0001 warning
  the fixture pins; it now reads through the alias instead, and its `.diag`
  golden stays W0001. `stdlib_text/builder_append_line.ark` moved its builder
  into `result` and then read `sb.len()`; the read now uses `result`, the new
  owner.

### Non-call use positions (2026-10-29)

The check is no longer limited to call arguments and receivers. Identifier
inference (`infer_leaf_expr` → `ownership::check_moved_binding_use`) applies
the same moved / `Copy` predicate to every identifier it evaluates, so a
direct `return` / tail value, a binary or unary operand, and the root of a
plain field or element read are E0212 like any other use. The existing
`consume_expr` move paths report the same span through
`report_ownership_error_once`, so a `let` initializer that reads and then
moves the same identifier still reports one E0212.

The callee of a call is not inferred as an expression (call resolution reads
the callee's spelling), so `infer_compound_expr` checks child 0 with the same
`check_moved_place_use` predicate. A call through a moved function value is
E0212; a direct module function call has no local binding and stays legal.

Move state remains the existing linear-scope model: a move recorded in an
earlier `match` arm or branch is visible to later arms/branches, so a read in
a later mutually exclusive path can be reported conservatively. That
precision limit already applied to `let`-initializer moves and call
arguments; the read check inherits it rather than adding a new flow analysis.
Identifiers in an `if` condition remain outside the check because condition
inference visits only call nodes for demand edges; that gap predates this
slice.

## Implementation (2026-10-29, non-call reads)

- `ownership::check_moved_binding_use` is the single expression-read entry
  point. It reuses `is_moved_binding` (binding index, `Copy`, moved) and
  `report_ownership_error_once` (code + span dedupe), and reports E0212 with
  the existing `move_after_use_message` hint. `infer_leaf_expr` calls it before
  marking the binding used, so return/tail values, operands, and field or
  element read roots need no shape-specific checks in the binary, unary,
  field, or return modules.
- `infer_compound_expr` checks the callee (child 0) of `NK_CALL` with
  `check_moved_place_use` before `check_call_place_uses`; method receivers stay
  in the existing call child-0 check.
- No double reporting: the `Box_new` / `unbox` consume path and the call checks
  already share `report_ownership_error_once`; a borrowed `Box<T>` parameter
  is never marked moved, so E0227 is unchanged; and
  `param_mode::check_mutable_parameter_places` already skips `is_moved_place`
  arguments, so a moved argument keeps one E0212 and gains no E0223.
- Selfhost migration: the stage-3-equivalent check (s2 `check` over the flat
  overlay with the workspace preopen) reported two move-after-use sites.
  `type_table.ark` moved `id` into `TypeEntry` and returned the original;
  `gc_layout_table.ark` did the same with `GcLayoutEntry`. Both now clone the
  id into the entry, and the same check reports zero errors (warnings only).
- Fixtures: `diagnostics/moved_return_value.ark`,
  `moved_tail_value.ark`, `moved_binary_operand.ark`,
  `moved_unary_operand.ark`, `moved_field_read.ark`, and
  `moved_function_callee.ark` are `diag:` entries. The positive
  `ownership/call_borrow_keeps_binding.ark` now also reads a field into a
  `let` initializer to show a live binding and its field stay readable.

## Verification (2026-10-29, call arguments)

- Build 1: `python3 scripts/manager.py selfhost build-compiler` PASS
  (1971.2s), s2 sha256
  `d95d8f0aa25e9c85e8f9af541ad9fadfc9898c7f283dff60f35907536cee77e2`.
- First `selfhost fixpoint` run caught a fourth selfhost site: stage 3
  reported E0212 for `key` in `gc_layout_table.ark` (a multi-line struct
  literal the initial line-based audit missed). It now clones the entry key.
- Build 2: `python3 scripts/manager.py selfhost build-compiler` PASS
  (2368.0s), s2 sha256
  `8d271ab3e26ca459bcd323c99c518711b421f11ebaa3ba1fbffa3b196e84f1e3`.
- `python3 scripts/manager.py selfhost fixpoint`: PASS — stage 3 was rebuilt
  from s2 and `sha256(s2) == sha256(s3)` =
  `8d271ab3e26ca459bcd323c99c518711b421f11ebaa3ba1fbffa3b196e84f1e3`
  (pinned base `8df2698c8401693879ca4f9ced289ed2c285d95eb526cf70fda02dd5876259ee`).
- `python3 scripts/manager.py verify fixtures`: PASS — PASS=1890 FAIL=0,
  COMPILE_VALIDATE=1890, EXECUTED=109, RUNTIME_UNSUPPORTED=2,
  `test:` PASS=36 FAIL=0.
- `python3 scripts/manager.py selfhost diag-parity`: PASS=194 FAIL=0
  (includes the five new `.diag` goldens).
- `python3 scripts/manager.py verify lane`: PASS — `quality changed`;
  lint checked=8 failed=0; lint ratchet checked=8 failed=0.
- `python3 scripts/manager.py docs check`: 6/6 PASS.
- Cross-backend parity for `ownership/call_borrow_new_owner.ark`
  (`scripts/check/check-cross-backend-parity.py --fixture ...`): parity=1;
  the wasm32-gc component and the wasm32 linear core both exit 0 and print
  `5/5/5`, matching `.expected`.
- The five negative fixtures were checked with the final s2 on both
  `--target wasm32-gc` and `--target wasm32`: each emits exactly one E0212
  and zero E0223, with the same message and span on both targets.
- Corpus migration: `diagnostics/mutable_sharing.ark` now reads through its
  alias (the old post-move `push` would be E0212 and would suppress its
  W0001 golden), and `stdlib_text/builder_append_line.ark` reads
  `result.len()` after moving `sb` into `result`.

## Verification (2026-10-29, non-call reads)

- Build 1: `python3 scripts/manager.py selfhost build-compiler` PASS (2190.2s),
  s2 sha256
  `3447060c65912f74cfb8dd0d0b955af316750019dca5bfc98ab0990edaa86979`.
- Stage-3-equivalent source check with that s2 over the flat overlay
  (`check src/compiler/main.ark` under the workspace preopen) found two E0212
  sites: `type_table.ark` and `gc_layout_table.ark` returned an `id` that had
  been moved into the entry. Both now clone the id into the entry; the same
  check reports 0 errors (unused-binding warnings only).
- Build 2 (triggered by the stale-s2 guard in `verify fixtures`): PASS; final
  s2 sha256
  `ca291f7b77043d50a2c396cd5399590b0c3d488b2010fd645d169c13b625c476`.
- `python3 scripts/manager.py verify fixtures`: PASS — PASS=1891 FAIL=0,
  COMPILE_VALIDATE=1891, EXECUTED=109, RUNTIME_UNSUPPORTED=2,
  `test:` PASS=36 FAIL=0.
- Six negative fixtures on `--target wasm32-gc` and `--target wasm32` with the
  final s2: each emits exactly one E0212 and zero E0223/E0227; the first
  diagnostic is `error[E0212|typecheck]` on both targets.
- Dedupe on the existing move fixtures: each `moved_argument_*` still emits
  one E0212 (zero E0223); `adt_move_after_use.ark` (Box_new/unbox) emits one;
  `ownership_move_after_use.ark` keeps four E0212, one per distinct use; and
  `diagnostics/mutable_sharing.ark` stays at zero errors with its W0001
  golden.
- `python3 scripts/manager.py selfhost fixpoint`: PASS — s3 was rebuilt from
  the final s2 and `sha256(s2) == sha256(s3)` =
  `ca291f7b77043d50a2c396cd5399590b0c3d488b2010fd645d169c13b625c476`
  (pinned base `8df2698c8401693879ca4f9ced289ed2c285d95eb526cf70fda02dd5876259ee`).
- `python3 scripts/manager.py selfhost diag-parity`: PASS (200 `diag:`
  fixtures, including the six new goldens).
- `python3 scripts/manager.py verify lane`: PASS — `quality changed`;
  lint checked=6 failed=0; lint ratchet checked=6 failed=0.
- `python3 scripts/manager.py docs check`: 6/6 PASS.
- `python3 scripts/manager.py fmt --check` on the five changed compiler
  sources and seven fixture sources: checked=12 failed=0.

### Close note (2026-10-29)

The acceptance gates above are green with the final s2
`ca291f7b77043d50a2c396cd5399590b0c3d488b2010fd645d169c13b625c476`
and the stage-3 fixpoint. `verify lane`, `verify fixtures`, `docs check`,
`diag-parity`, and `fixpoint` were run in this worktree.

`python3 scripts/manager.py verify quick` reports 149/157 checks. Its eight
failures are pre-existing on the base tree and do not touch this issue or the
changed files: the playground close gates cannot run `npm install` in this
read-only sandbox (#472/#500); #932 has `Status: open` under `issues/done/`
and its evidence rules fail (with #814/#825/#927/#929/#942); the
`skip-doc-check` budget in `docs/compiler/error-codes.md` is unchanged at 11
and pre-dates this slice; `docs/stdlib/modules/process.md` lacks the mixed
availability axis; the ark-code-quality long-line ratchet is 10 > 2 with none
of the cited files changed here; and the constant-like-function table scan
cites `emit_rung_census.ark` / `helpers_io_p2.ark`, which are unchanged.
