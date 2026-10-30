---
Status: open
Created: 2026-10-27
Updated: 2026-10-30
ID: 936
Track: stdlib
Depends on: "#934"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#934"
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-042, ADR-046, RFC-005, #917, #928, #933, #935"
---

# 936 — prelude core functions have placeholder signatures; checker and emitter dispatch them by name

## Summary

Core collection / string functions in `std/prelude.ark` declare signatures
that do not describe their real contract, and their bodies are
`panic(...)`. The comment says "polymorphic in practice; emitter dispatches
by name". The typechecker special-cases these names and skips argument
checking; the emitter recognizes the names and emits intrinsics. As a
result, ill-typed calls are accepted and become memory reinterpretation on
`wasm32` and invalid Wasm on `wasm32-gc`.

## Evidence (2026-10-27)

- `std/prelude.ark` L663–700: `pub fn len(s: String) -> i32`,
  `pub fn push(v: Vec<i32>, val: i32)`, `pub fn get(v: Vec<i32>, idx: i32)`,
  `pub fn is_empty(v: Vec<i32>)`, each with a `panic(String_new())` body.
  L79–84: `pub fn clone(s: String) -> String` is used for every type (#928).
- `src/compiler/typechecker/builtins.ark` `builtin_call_return_type`
  returns `i32` for any `len(...)` and `TY_UNKNOWN` for unknown names; the
  arguments are not checked.

Repro 1 (`ty_push.ark`):

```ark
use std::host::stdio

fn main() {
    let v = Vec::new<i32>()
    push(v, String_from("oops"))
    stdio::println(len(v))
    let x: i32 = get_unchecked(v, 0)
    stdio::println(x)
}
```

Repro 2 (`ty_len.ark`):

```ark
use std::host::stdio

fn main() {
    let n = 42
    stdio::println(len(n))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`):

- `wasm32`: both compile; `wasmtime run` prints `1` / `140` (the string
  pointer read as `i32`) and `0`.
- `wasm32-gc`: both report "compiler produced invalid core Wasm".

## Expected

`push(Vec<i32>, String)` and `len(i32)` are compile-time type errors. Core
operations have real signatures (generic functions or trait methods per
ADR-046), and backend lowering is bound to them through FunctionId →
CoreOp (ADR-042), not through the call-site spelling.

## Acceptance

- [ ] No prelude function declares a placeholder signature; the remaining
      `panic` placeholder bodies are replaced by real bodies or by sealed
      CoreOp bindings.
- [ ] The typechecker has no name-keyed return-type or argument exemptions
      for these operations (`builtins.ark`, `call_method.ark`,
      `vec_core_mono.ark`, `hashmap_methods.ark` name lists removed or made
      signature-driven).
- [ ] The emitter selects intrinsics through the resolved FunctionId / CoreOp,
      not by callee-name prefix (`starts_with(callee, "push__")` etc.).
- [x] Compile-error fixtures for both repros pass on `wasm32` and
      `wasm32-gc`.
- [ ] #928 (`clone(Vec<T>)` on `wasm32-gc`) is fixed by the same change.

## Update (2026-10-30, `wave/issue-936`) — signature-driven Vec free checks

Landed the first increment: the stable Vec prelude frees no longer take an
unchecked, name-only path in the typechecker.

- `std/prelude.ark`: `push<T>(mut Vec<T>, T)`, `get<T>(Vec<T>, i32) -> Option<T>`,
  and `pop<T>(mut Vec<T>) -> Option<T>` now declare the contract the manifest
  already records (`Vec<T>` + element type), so `push(Vec<i32>, String)` is
  E0200 at the call instead of memory reinterpretation/invalid Wasm.
  `get_unchecked<T>` / `set<T>` already had real generic signatures.
  Generic argument inference also adapts an unsuffixed integer literal to an
  already-bound `i64` parameter (`push(Vec<i64>, 10)`), so the stricter
  signatures do not reject valid literal calls; typed `i32`/`i64` values still
  follow ADR-067.
- `src/compiler/typechecker/call_fn.ark`: `len` is the one stable free with
  two owners (`String`, `Vec<T>`). Its declaration must keep the String
  spelling until the committed pin is refreshed (the pin compiles the source
  tree and would reject `len(String)` against a Vec-only declaration), so the
  checker selects the receiver's owner contract: `String` and `Vec<T>` return
  `i32`, any other concrete first argument is E0200
  (`argument 1 has type i32, expected String or Vec<T>`). The
  `builtin_call_return_type` `len` → `i32` name exemption is removed.
- Fixtures: `diagnostics/prelude_vec_push_arg_type.ark` and
  `diagnostics/prelude_len_arg_type.ark` (plus `_wasm32` companions with
  `--target wasm32 --wasi-version wasi-p1`) assert the two repros fail on both
  targets; `stdlib_vec/prelude_generic_signatures.ark` runs `len` on `Vec` and
  `String` plus generic `push`/`get`/`pop` on both targets.

## Update (2026-10-30, `wave/issue-936-2`) — `len` owner must be decided by the end of the body

The first increment still accepted a `len` receiver whose type was a type
variable, because the variable might bind to `String` or `Vec<T>` later in the
body. That also let an unconstrained `T` reach lowering, where bare `len` is
bound to `string.len` by name, instead of being rejected.

- `src/compiler/typechecker/call_fn.ark`: a `len` call on a type variable now
  records the receiver on `TypeEnv.deferred_len_receiver_types` instead of
  accepting it. `finalize_deferred_len_receivers` resolves each receiver after
  the body has run: `String` / `Vec<T>` pass (a later binding may still supply
  the owner), a still-unresolved variable is E0200
  ("cannot infer the type of the `len` receiver: argument 1 must be String or
  Vec<T>"), and a variable that resolved to another concrete type reuses the
  call-site mismatch message. `TY_UNKNOWN` receivers stay quiet because they
  are error-recovery values; the general unknown-type rule stays with #939.
- `src/compiler/typechecker/body.ark`: function and test bodies finalize the
  deferred receivers right after statement checking. The return-contract pass
  re-infers the body afterwards; those additional entries are discarded with
  the per-function env.
- Fixtures: `diagnostics/prelude_len_unconstrained_receiver.ark` and its
  `_wasm32` companion assert the unconstrained-`T` case on both targets.
  `stdlib_vec/prelude_generic_signatures.ark` now also runs `len` inside a
  generic `Vec<T>` helper for `Vec<i32>` and `Vec<String>`.
- The deferred-accept path (a fresh receiver variable that a later call binds
  to `String`) has no run fixture: a `len` inside an unannotated closure still
  emits invalid Wasm because lowering re-infers the parameter as an integer
  and dispatches `string.len` by name. The pinned compiler fails the same way,
  so this is the existing #933/#934 lowering gap, not a regression from the
  checker finalizer.

Remaining acceptance work (updated):

- The emitter still selects the intrinsic through the callee-name tables.
  Bare `len` is bound to `string.len` in `data/core-ops.toml`, and MIR rewrites
  it to `raw_array_len` only from the first argument's local type; `push` /
  `get` / `pop` still lower through `call_rewrite_vec*` name/predicate
  dispatch. Binding the call to the resolved `FunctionId` → `CoreOpId` needs
  the #933 handoff.
- `len`, `push`, `get`, and `pop` still have `Std::panic`/placeholder bodies.
  They are deferred signature-only decls, not sealed CoreOp bindings.
- The typechecker still keeps name-keyed tables for concrete prelude
  conversions (`builtins::is_concrete_prelude_builtin`,
  `builtin_call_return_type`) and for `eq`/`assert_eq_i64`; those are outside
  this increment's Vec-operation scope.
- #928 is not changed by this diff; it remains tracked separately.

## Update (2026-10-30, `wave/coreop-seal`) — `len` binds through the receiver owner's CoreOp

Landed the first FunctionId → CoreOpId step for `len` (the #933 handoff used by
this issue's acceptance item 3):

- `data/core-ops.toml`: the bare `len` alias is removed. The registry binds the
  two owner spellings instead: `String::len` → `string.len` and `Vec::len` →
  `vec.len`. A call site now reaches the operation through the owner of the
  receiver, not through the common callee name.
- `src/compiler/mir/lower/call_rewrite_vec.ark`: MIR `len` lowering returns the
  owner spelling (`Vec::len` for `Vec` / `vec:` / `array:` receivers,
  `String::len` otherwise) instead of rewriting Vec receivers to the target-raw
  `raw_array_len`. Both owner spellings are treated as intrinsic targets so mono
  resolution cannot rename them.
- `src/compiler/mir/lower/stdlib_body_defer.ark`: the `len` declaration stays
  deferred after losing its CoreOp alias, so it remains a signature-only decl
  instead of lowering its `Std::panic` placeholder body.
- `src/compiler/mir_opt/stdlib_inline.ark`: the normal-call fallback resolver
  no longer specializes `__core_vec_len_impl` to `__core_vec_len_impl__i32`.
  Reachability prunes that mono body as an intrinsic call, so specializing the
  call left an unpatched call and trapped at runtime; `vec.len` now stays on its
  registry handler, which emits the length for every element type.
- Fixtures: `stdlib_vec/prelude_len_owner_dispatch.ark` and its `_wasm32`
  companion (`--target wasm32 --wasi-version wasi-p1`) run `len` on `String`,
  `Vec<i32>`, `Vec<String>`, empty vectors and a generic `Vec<T>` helper on both
  targets. Expected output: `len owner dispatch ok`.

Evidence: in the linear-memory WAT for the fixture the emitted `String::len`
body is no longer called (the call is inlined as the `string.len` intrinsic),
and the compiler's `core-op-shadow` spine audit reports
`mismatched=0 unresolved=0` for both targets. `verify lane`,
`selfhost fixture-parity --filter-dir stdlib_vec`, `verify fixtures`,
`docs check`, `selfhost diag-parity` and `selfhost fixpoint --build` stay
green.

Acceptance movement: item 3 moves for `len` only (the emitter now selects the
intrinsic from the owner-qualified FunctionId → CoreOpId). Items 1, 2 and 4 are
unchanged. Residuals:

- `push`, `get` and `pop` still lower through the `call_rewrite_vec*`
  element-type predicates, and `get` / `pop` / `set` are still listed in
  `mir_is_unbound_prelude_intrinsic_shim`.
- `len`, `push`, `get` and `pop` still declare signature-only bodies.
- The `len` owner is still selected from the MIR local type spelling, not from a
  checker-supplied per-call FunctionId (#933); a receiver whose type lowering
  cannot see keeps the declaration's `String` contract.

## Update (2026-10-30, `wave/functionid-handoff`) — checker-owned per-call FunctionId for `len` and `pop`

The owner is no longer recovered from the MIR local type spelling. The
typechecker records a per-call receiver resolution on the checked program and
MIR lowering binds the owner-qualified FunctionId to its CoreOp from that
record:

- `TypeCheckResult.resolved_call_owners` carries
  `{decl_index, body_child_index, span_start, callee_name, owner_name}`. The
  body identity is the source decl index plus the impl-method ordinal, so a
  call in a loaded module cannot alias a same-span call in another file.
  `CoreHirResolvedCall` now carries this resolution through `CoreHirMirView`
  into the lowering ctx; the old syntactic `scan_resolved_calls` is deleted.
- `len`: `infer_prelude_len_call` records `String` / `Vec` from the unified
  receiver type (including deferred type-variable receivers after body
  finalization). MIR `mir_rewrite_vec_len` reads the record and emits
  `String::len` / `Vec::len`; it no longer reads `ctx_get_local_type`, and a
  site with no record keeps its callee instead of guessing String.
- Generic free calls (`push`, `get`, `pop`, and any Vec/String-first-param
  generic) record the owner of the instantiated first parameter, even when the
  element type is still an open type variable.
- `pop`: MIR emits the owner-qualified `Vec::pop` from the record; the new
  `Vec::pop` → `vec.pop` registry alias binds that FunctionId to the CoreOp.
  `push` / `get` still lower through `call_rewrite_vec*` element-type
  predicates; their owner records are in place for the next increment.
- Fixtures: `stdlib_vec/prelude_call_owner_untracked_receiver.ark` and its
  `_wasm32` companion use an if-expression receiver whose MIR local has no
  element type. Before this change the linear target emitted
  `__core_string_len_impl` for `len(values)` and printed the Vec pointer as a
  length; both targets now emit `Vec::len` / `Vec::pop`.

Evidence: MIR dumps for both targets show `callee=Vec::len` and
`callee=Vec::pop` with registry-bound fids; both fixture targets print
`prelude call owner untracked receiver ok`.

Acceptance movement: item 3 moves for `len` and `pop` (the emitter selects
those intrinsics from the checker-resolved owner-qualified FunctionId →
CoreOpId). Items 1, 2 and 4 are unchanged. Residuals:

- `push` and `get` still lower through the `call_rewrite_vec*` element-type
  predicates (owner records exist but MIR does not consume them yet), and
  `get` / `pop` / `set` are still listed in
  `mir_is_unbound_prelude_intrinsic_shim`.
- `len`, `push`, `get` and `pop` still declare signature-only bodies.
- The remaining name-keyed checker tables (`vec_core_mono`,
  `is_concrete_prelude_builtin`) and #928 are unchanged.

## Update (2026-10-30, `wave/push-get-owner`) — owner handoff for `push` and `get`

Completes the item 3 movement for the remaining Vec frees in the handoff set.
`push` and `get` now select their lowering from the checker's per-call owner
record like `len` and `pop`; the element-type predicates still choose the
concrete numeric intrinsic, but they no longer decide whether a bare callee is
the prelude free.

- `data/core-ops.toml`: new owner-qualified aliases `Vec::push` → `vec.push`
  and `Vec::get` → `vec.get`. The bare `push` alias stays only for the
  migration table (method/spine spellings and the deferred declaration body)
  until #818 removes it.
- `src/compiler/mir/lower/call_rewrite_vec_push.ark`: a bare `push` reaches the
  element-type rewrite only when `ctx_resolved_call_owner(span, "push")`
  reports `Vec`; a site without a record keeps its callee. Owner-resolved
  calls whose receiver local has no element type emit `Vec::push`; a receiver
  that carries one keeps the historical element-typed spelling (`vec_push`,
  `push_i64`, `push_f64`, including the i32→i64 widen), and the method-call
  rewrite (span 0) keeps that same element-type path.
- `src/compiler/mir/lower/call_rewrite_vec.ark`: `get` consults the record the
  same way and emits `Vec::get` when the receiver local has no element type,
  or the element-typed `vec_get` when it does. `Vec::push` / `Vec::get` are
  intrinsic targets so mono resolution cannot rename them; `Vec::get` takes
  the same `Option` dest type as `vec_get`.
- Fixtures: `stdlib_vec/prelude_push_get_owner_untracked_receiver` and its
  `_wasm32` companion (`--target wasm32 --wasi-version wasi-p1`) mirror the
  len/pop untracked fixture: an if-expression receiver whose MIR local loses
  its element type on the linear target. `get` on that local emits `Vec::get`
  on `wasm32`; the GC local keeps its element type, so the same call keeps
  `vec_get`.

Evidence: with the owner trace enabled, the linear companion's untracked
receiver rewrites to `Vec::push` / `Vec::get`; the concrete receivers keep
`vec_push` / `vec_get`, and the GC local keeps its element type, so its `get`
stays `vec_get`. The final compiler's core-op shadow audit reports
`candidates=14 matched=14 mismatched=0 unresolved=0` on both targets, and both
fixtures print `prelude push/get owner untracked receiver ok`.

Gates: verify fixtures 1940/1940 (41/41 test fixtures), verify lane, docs
check 6/6, selfhost diag-parity, selfhost fixpoint
(sha256 9dec894d3de5f69f28b39924e9d2e54fab004cf4a86b008aaa84f2e067eb5a23).

Acceptance movement: item 3 moves for `push` and `get`; items 1, 2 and 4 are
unchanged. Residuals:

- `len`, `push`, `get` and `pop` still declare signature-only bodies (item 1).
- The remaining name-keyed checker tables (`vec_core_mono`,
  `is_concrete_prelude_builtin`) and #928 are unchanged (item 2).
- The bare `push` alias and the wasm callee-name fallback (`call_*`) remain
  until #818; the element-typed `vec_push` / `push_i64` / `push_f64` /
  `vec_get` spellings are still selected from the receiver's element type
  because the linear emitter has dedicated numeric intrinsics.
- Method-call `.push` / `.get` rewriting (span 0) keeps its historical
  receiver-type path.

## Update (2026-10-30, `wave/checker-table-retire`) — `vec_core_mono` retired

The checker no longer maps Vec call spellings to CoreOp fallback symbols.
`src/compiler/typechecker/vec_core_mono.ark` and both call sites are deleted
(`infer_call_expr`'s unresolved-callee branch and
`infer_builtin_vec_method`).

What replaced the table:

- `len` / `pop` / `push` / `get` already lower from the per-call owner record
  (`ResolvedCallOwner` → owner-qualified FunctionId → CoreOp); the checker no
  longer needs a second, name-keyed mono demand for them.
- `set` / `get_unchecked` are checked through the prelude generic signatures
  and MIR rewrites them to the `vec.set` / `vec.get_unchecked` CoreOps.
- Vec method calls resolve through the `Vec::<method>` shell signature before
  the builtin Vec fallback, and `is_empty` lowers to the sealed raw length.
  The registry handlers in `wasm/call_vec.ark` emit `vec.len` /
  `vec.is_empty` / `vec.set` / `vec.get_unchecked` directly, and
  `mir_opt/stdlib_inline.ark` already refuses to specialize
  `__core_vec_len_impl`, so the deleted hint had no consumer left.

Evidence (current s2 = `5bcc29abf458…`):

- `selfhost build-compiler`: PASS, 55.3s.
- `verify fixtures`: PASS=1941 FAIL=0 COMPILE_VALIDATE=1941 EXECUTED=112
  RUNTIME_UNSUPPORTED=2; `test:` PASS=42 FAIL=0.
- Negative fixtures unchanged on both targets (`wasm32-gc` and the `_wasm32`
  companions with `--target wasm32 --wasi-version wasi-p1`):
  `prelude_len_arg_type` → "type mismatch in call to `len`: argument 1 has
  type i32, expected String or Vec<T>"; `prelude_vec_push_arg_type` → "type
  mismatch: i32 vs String"; `prelude_len_unconstrained_receiver` → "cannot
  infer the type of the `len` receiver: argument 1 must be String or Vec<T>".
- `verify lane`, `docs check` 6/6, `selfhost diag-parity` and `selfhost
  fixpoint` pass; fixpoint s2 == s3 =
  `5bcc29abf45821072834f2d882f54bca579616e4939f759322289f9626c509cb` on pin
  `56ff0808171dcc3c521b0e85d07f2b16a349a59419483ad71148a5ee928d23e8`.
  Stage 3 is the s2 self-build of the trimmed bootstrap overlay, where the
  builtin Vec fallback is the live path, so the fixpoint also covers the case
  the deleted table was written for.

Acceptance movement: item 2's `vec_core_mono` entry is gone; items 1, 3 and 4
are unchanged. Remaining name-keyed tables and their blockers:

- `builtins::is_concrete_prelude_builtin` and `builtin_call_return_type` stay
  until #935 lands general concrete-call argument unification. #935 forbids a
  new per-name allowlist, and `assert_eq_i64` (emitter i32→i64 widening) and
  `char_to_string` (i32 codepoint representation) still rely on the current
  exemptions.
- `call_method::infer_builtin_vec_method` still name-keys the Vec fallback
  return types and arity (`len` / `is_empty` / `pop` / `get` /
  `get_unchecked`). It is a result-type table used when the `Vec::*` shell
  signature is absent (trimmed bootstrap overlay), not a CoreOp dispatcher.
- MIR still chooses concrete element intrinsics by name predicate
  (`call_rewrite_vec*`, `stdlib_specialize_vec_fallback`) and keeps
  `get` / `pop` / `set` in `mir_is_unbound_prelude_intrinsic_shim`; the bare
  `push` alias stays until #818.
- `len` / `push` / `get` / `pop` still declare signature-only bodies (item 1);
  the `len` declaration keeps the String spelling until the committed pin is
  refreshed. #928 is unchanged.

## Update (2026-10-30, `wave/vec-method-fallback`) — `infer_builtin_vec_method` retired

The method fallback is now signature-driven. `call_method.ark` no longer owns a
name-keyed return-type/arity table for Vec methods; it resolves the call
against a real declaration and runs the ordinary method-call checking.

- `src/compiler/typechecker/call_method.ark`: `infer_builtin_vec_method` and
  `is_vec_method_with_arity` are replaced by `infer_vec_method_from_signature`.
  For a `Vec` receiver with no `Vec::<method>` shell signature it looks up, in
  order: the free declaration of that name whose first parameter is `Vec<T>`
  (`push` / `pop` / `get` / `set` / `get_unchecked`), then the CoreOp
  implementation-symbol declaration `__core_vec_<method>_impl` (which covers
  `len` and `is_empty`). The resolved FnSig binds the receiver's element type,
  instantiates the return type (`Option<T>`, `T`, `i32`, `bool`), checks
  arity and argument types, and records the same mono demand as a normal
  generic method call. Arity mismatches keep the historical fall-through so
  this change does not add a new diagnostic surface.
- `src/compiler/typechecker/call_generic_method_args.ark`: generic method
  arguments now apply the same unsuffixed-integer adaptation as generic free
  arguments, so `v.push(10)` on `Vec<i64>` keeps the documented #917 behavior
  under the stricter signature check.
- No std or overlay change is needed: the declarations the lookup uses
  (`push` / `pop` / `get` / `set` / `get_unchecked`,
  `__core_vec_len_impl`, `__core_vec_is_empty_impl`) already live in
  `std/prelude.ark`, which both user builds and the trimmed bootstrap overlay
  ship.
- Fixtures: `stdlib_vec/prelude_vec_method_signatures` (plus the `_wasm32`
  companion) exercises all seven methods and the i64 literal push without the
  extension import; `diagnostics/prelude_vec_method_arg_type` (plus `_wasm32`)
  asserts `values.push(String_from("oops"))` on `Vec<i32>` is E0200 now that
  the method resolves through a receiver-bound signature.

Why the `Vec::*` shell alone was not enough: inherent impl signatures keep an
untyped `self` for Vec (the MIR adapter path), so the shell cannot bind the
element type from the receiver. The signature-driven lookup uses the
declarations that do carry `Vec<T>` as their first parameter.

Evidence (current s2 = `de70df8d9d0f…`):

- `selfhost build-compiler`: PASS, 65.7s.
- `verify fixtures`: PASS=1945 FAIL=0 COMPILE_VALIDATE=1945 EXECUTED=112
  RUNTIME_UNSUPPORTED=2; `test:` PASS=42 FAIL=0.
- `verify lane`, `docs check` 6/6, `selfhost diag-parity` pass.
- `selfhost fixpoint`: s2 == s3 =
  `de70df8d9d0fda4eebe7bf9e53f8018d1389eeefc688a09fd97b374ecc7c3c03`
  on pin
  `56ff0808171dcc3c521b0e85d07f2b16a349a59419483ad71148a5ee928d23e8`.
  Stage 3 is the s2 self-build of the trimmed bootstrap overlay, so the new
  resolver is exercised on the whole compiler source (the Vec method calls in
  the checker/MIR are the live fallback path there).
- Targeted fixture evidence: the negative fixture prints
  `type mismatch: i32 vs String` on `wasm32-gc` and `wasm32`; the positive
  fixture prints `prelude vec method signatures ok` on both targets.
- Generated docs regenerated for the five new manifest entries (fixture
  registry 3603 → 3608, remainder 2015 → 2020).

Acceptance movement: item 2's `call_method.ark` entry is gone; items 1, 3 and
4 are unchanged. Remaining name-keyed tables and blockers:

- `builtins::is_concrete_prelude_builtin` / `builtin_call_return_type` stay
  until #935 lands general concrete-call argument unification; `assert_eq_i64`
  and `char_to_string` still rely on those exemptions.
- MIR still chooses concrete element intrinsics by name predicate
  (`call_rewrite_vec*`, `stdlib_specialize_vec_fallback`) and keeps
  `get` / `pop` / `set` in `mir_is_unbound_prelude_intrinsic_shim`; the bare
  `push` alias stays until #818.
- `len` / `push` / `get` / `pop` still declare signature-only or deferred
  bodies (item 1); the free `len` declaration keeps the String spelling until
  the committed pin is refreshed. #928 is unchanged.

## Notes

#917 is converting stdlib free functions to methods; this issue owns the
type-contract part (signatures must be true) so that #935 can be enabled.
