---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 872
Parent: 852
Track: language-v2
Depends on: "855, 859, 860, 871"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 8 (closure/collection)"
---

# 872 — Wasm GC closure / collection lowering を完成する

## Scope

RFC-011: **8 (closure/collection)**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] closure env を Wasm GC managed representation にする
- [ ] Vec/String/HashMap の managed parts を GC representation にする
- [ ] generic element boxing policy を固定する
- [ ] runtime helper 依存を最小化する
- [ ] representative compile/run/validate fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する

## Progress receipt — 2026-09-20 (closure annotation typing prerequisite)

- Commit `104b108d2` preserves closure parameter annotations and the optional
  return annotation in the selfhost AST. Closure inference now uses annotated
  parameter types before operator-trait checks and unifies an annotated return
  type with the body type; unannotated closures retain fresh-variable
  inference.
- `benchmarks/bench_cpu_closure_map.ark` is a positive regression workload with
  two typed capturing closures. The current s2 compiler produces a valid GC
  component (3447 bytes) and a valid linear-memory core module (3099 bytes).
  Both validate and run under Wasmtime with the expected output `6017000`.
- `python3 scripts/manager.py verify lane` passed, including formatter,
  quality contract, lint smoke, and lint ratchet. This receipt covers the
  typing/parser prerequisite only; managed closure environments, collection
  lowering, and the representative negative fixture remain open, so #872 is
  not closed.

## Progress receipt — 2026-09-25 (capture-escape diagnostic E0220)

- Commit `865aca4d5` turns every capturing-closure escape into a compile-time
  diagnostic instead of the previous invalid-Wasm / runtime-trap outcome.
  Borrow captures keep the semantic error E0213; move and Copy captures —
  legal per ADR-061 but not yet implemented as managed environments — get the
  new capability diagnostic E0220, whose removal condition is this issue's
  managed-env lowering.
- Covered positions: call and method-call arguments plus method receivers
  (chokepoint in `infer_call_expr` / `infer_method_call_expr`, so unresolved
  and builtin calls are checked identically), explicit `return`, block tail
  expressions, `let` init expressions that are not a literal closure (alias,
  block, if/match, aggregates — only a literal init registers the binding for
  capture supply), tuple-destructure init, and assignment right-hand sides.
  Bindings track `has_captures` so aliased values stay rejected downstream.
- Verified on s2 (`selfhost build-compiler`): `clo_esc`/`clo_ret_branch`/
  `clo_hof`/`clo_map`/`probe_*` now fail with E0220 at the escape site,
  `clo_map_str` keeps E0213, and the existing diag fixtures
  `closure_borrow_{callback,conflict,return,store}` and
  `closure_move_after_capture` still produce E0213/E0214/E0212. Positive
  cases that must keep working do: `clo_cap` prints 15, non-capturing
  `docs/examples/closure.ark` prints 42, and
  `benchmarks/bench_cpu_closure_map.ark` (direct calls only) prints 6017000.
- `python3 scripts/manager.py verify lane` passed (comment policy,
  editorconfig, quality contract 24 rules, touched-code ratchet, lint
  checked=13 failed=0). `src/compiler/diagnostics/codes.ark` had its
  pre-existing `pub pub pub` corruption repaired so it parses under the
  canonical formatter; its `docs/data/ark-formatter-baseline.toml` exception
  was removed.
- Residuals: `let g = f` rebinding through a *new* let is rejected, but
  re-assignment `g = f` where `f` has captures is flagged at the assignment
  while a pre-existing `g` binding keeps its flags only approximately;
  `for`-source and comprehension positions inherit coverage through the
  call chokepoint. Managed closure environments (the actual acceptance
  criterion), collection GC lowering, and the representative fixture remain
 open, so #872 is not closed. The same diagnostic now fires for the
 linear-memory target, which shares the bare-funcref limitation (#874).

## Progress receipt — 2026-09-25 (Copy-capture snapshot at literal)

- Implementation commit `da7296a13` (worktree-verified as `1d3a443bf`)
  fixes a spec divergence found while scoping env lowering: the lowering
  packed source-local names as call-supply captures, so a Copy binding
  reassigned between closure-literal evaluation and the call leaked its
  new value into the closure. ADR-061 shared-copy capture requires the
  capture-time value (spec: "Copy bindings are copied independently of
  the capture mode"). Scalar Copy captures (i32/i64/f64/f32/v128) are now
  snapshotted into hidden `__capsnap_<synth>_<i>` locals emitted at the
  literal position; the packed supply names point at the snapshots while
  the closure body keeps the original names for its capture params.
  Non-Copy captures keep the source-local name — borrow captures read
  the live local (correct: they cannot escape, E0213) and move captures
  read a slot whose source is already dead. VT_FUNCREF captures were
  deliberately left on the live-read path: the typechecker treats `fn`
  values as non-Copy borrows (E0214 fires on reassignment), so live-read
  is the conforming semantics.
- Positive fixture: `tests/fixtures/closure_capture/copy_capture_snapshot.ark`
  prints `5` (closure sees capture-time value after `y = 10`) then `10`
  (a second literal sees the reassigned value); registered as `run:`,
  `t3-compile:`, and `t3-run:`.
- Verification: wasm32-gc component and wasm32 linear runs of the probe
  print `5`/`10`; all existing closure fixtures unchanged
  (`capture_struct` 35/hello 42, `borrow_local`, `move_local`,
  `bench_cpu_closure_map` 6017000, `docs/examples/closure.ark` 42);
  escape diagnostics unchanged (move-capture escape E0220, borrow
  escape E0213); `verify lane` PASS on the worktree; fixture gate
  `closure_capture` 4/4 compile+validate+execute PASS on both the
  worktree and main-tree s2 rebuilds.
- Design note for the remaining env work: this slice introduces the
  capture-slot discipline a managed env needs — env slots are the same
  capture-time values. The uniform pair ABI (`fn` value =
  `{funcref, env}` with env as hidden arg0) is the next slice; it must
  also cover named-function coercion thunks (verified today
  `apply(add1, 41)` works via bare `ref.func`).
- Residuals: i64/f64 captures already produce invalid core Wasm on
  master (pre-existing emitter bug, reproduced without this change);
  the snapshot covers them so the fix inherits correct semantics.
  Managed env representation, `fn` pair ABI, mutable-borrow env cells,
  and the representative escape fixture remain open; #872 stays open.

## Progress receipt — 2026-09-25 (managed {fnptr, env} pair ABI)

- Commit `859a721fb` (worktree-verified as `87daa6524`) lands the
  uniform pair ABI: every `fn` value is a `__fnpair {funcref, structref}`
  record. `__fnpair` is a fixed GC type (`gc_fnpair_offset` = 26) so
  `Vec<fn>`/`HashMap<_,fn>` element arrays reference it as a backward
  type index, and lazy named-struct registration dedups to the same
  canon index — signatures, locals, fields, and container elements all
  resolve to one storage type.
- Escaping move closures keep a shared per-literal env struct; direct
  calls keep the `(params..., captures...)` signature while fn-typed
  calls go through synthesized env trampolines (`struct.get env` →
  args → `struct.get fnptr` → `ref.cast` → `call_ref`). Named-fn
  coercion produces `__fnthunk_<name>` pairs; closure params now read
  their type annotations instead of hardcoded VT_I32.
- Fixed-offset renumbering: inserting `__fnpair` at offset 26 shifted
  `A_fnref`→27, Vec<fn> header→28, `A_ref14`→29, Vec<enum>→30,
  HashMap<i32,String,fn>→31/32; all sig strings, layout-plan offsets,
  MIR `_refN` suffixes, and emit-side offset helpers updated to match.
- `inst_store` treats `CALL_REF` as a stack call for tee/drop
  decisions — call_ref results feeding println/next-call args were
  SKIP-stored leaving null dest locals (runtime null-ref trap).
- Mutable-rebind captures that escape now error `E0213` (honest
  fallback per ADR-061 until env cells exist); bare-ident rebind targets
  are pinned to `capture_mutable_borrow`. Copy/move captures escape
  freely — `move || { v.push(x) }` and field writes stay move
  captures since the shared env ref persists writes.
- Fixture conversions: stale E0220 diagnostics
  `closure_capture_escape_store`/`_arg` became positive run fixtures
  `closure_capture/escape_store.ark` (15) and `escape_arg.ark` (121);
  `closure_capture_escape_return` stays diag as `E0213`.
  `fixture_manifest_count` synced to 3006.
- Verification: `vec_fn_push_call` prints 10; fixture gates
  `closure_capture` 6/6, `functions` 15/15, `collections` 14/14,
  `enums` 10/10, `generics` 7/7, `scalar` 19/19, `control` 13/13
  PASS on both worktree and main-tree s2 rebuilds; `verify lane` PASS
  (comment policy, editorconfig, quality contract, ratchet, lint 37/0).
- Residuals: mutable-rebind env cells still pending (counter-style
  escape → E0213 until cells exist); `fn()->String` and ≥2-param fn
  type names still bucket to `fnref`/`fnref2` losing the exact
  signature; `closure_param_type_is_fnref` is duplicated across two
  lowering files; linear-memory pair path is implemented but not yet
  exercised end-to-end (#874 owns parity); env allocation happens per
  literal evaluation (no caching). `ownership/adt_probe`,
  `match_extensions/struct_pattern`, `question_mark/error_ecosystem`,
  `generics_v1/test_enum_return` fail identically on master —
  pre-existing, unrelated to this change. #872 stays open pending
  linear parity (#874) and cross-backend parity evidence (#875).
