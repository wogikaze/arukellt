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
