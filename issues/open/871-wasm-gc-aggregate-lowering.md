---
Status: open
Created: 2026-09-19
Updated: 2026-09-20
ID: 871
Parent: 852
Track: language-v2
Depends on: "854, 870"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 8 (aggregate)"
---

# 871 — Wasm GC aggregate lowering を完成する

## Scope

RFC-011: **8 (aggregate)**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] struct/tuple/enum/Option/Result を Wasm GC types/instructions へ lower する
- [ ] tag/payload representation を実装する
- [ ] primary target の silent fallback を binary inspection で防ぐ
- [ ] `wasm-tools validate` を通す
- [ ] value semantics と representation optimization を分離する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Progress receipt (not done; 2026-09-20)

- Verification guard: `b57a3f670` adds
  `scripts/tests/test_wasm_gc_aggregate_lowering.py`; `c11c782fc` extends it
  to runtime parity. It compiles the
  aggregate fixtures through the current selfhost compiler for the default
  `wasm32-gc` P2/Memory64 path and the `wasm32` P1/memory32 fallback, then
  runs `wasm-tools validate` and inspects the printed binary. The primary
  artifact must contain concrete `struct.new/get/set` or `array.new/get/set`
  instructions; the fallback artifact must contain no `struct.*`, `array.*`,
  or `ref.*` instructions.
- Positive fixtures cover a named enum payload in a struct/Vec, `Option`,
  `Result`, and tuple lowering. The same-source linear artifact is the
  negative representation check for silent GC fallback.
- Runtime parity: the GC artifact is packaged as a P2 component and the
  fallback artifact as a P1 wasm module. Both are executed with Wasmtime and
  each target's stdout matches the fixture `.expected` output for all four
  fixtures.
- Verification receipts: the focused test passes (`Ran 1 test in 47.071s`),
  the extended combined linear-runtime and aggregate suite passes (`Ran 5
  tests in 53.082s`), and `python3 scripts/manager.py verify lane` passes its quality
  gates. `git diff --check` and the staged diff check also pass.
- Normative boundary: the representation/meaning separation remains defined
  by RFC-011 item 8 and ADR-060; this test adds executable binary evidence and
  does not introduce a new semantic or ABI decision.
- Range scalar-width receipt: `38cc7cc71` fixes the existing `Range` GC
  intrinsic at the Memory64 aggregate boundary. `range_new` narrows the
  i64 scalar ABI values before constructing the two-i32 GC struct; `range_len`
  and `range_contains` use width-aware arithmetic/comparisons after
  `struct.get`, and widen the boolean result where the Memory64 scalar ABI
  requires it. The change reuses the existing numeric-width helpers and does
  not alter range meaning or the linear-memory representation.
- Range verification: the temporary probe covering i64-to-i32 inputs,
  `range_new`, `range_len`, and `range_contains` passed compile, validation,
  and runtime on `wasm32-gc` P2/Memory64 and `wasm32` P1/memory32. The tracked
  `stdlib_core/range_len_empty.ark` and
  `stdlib_core/range_contains_edges.ark` fixtures also passed the same
  compile/validate/runtime matrix. The GC artifacts were validated with
  `wasm-tools validate --features gc,function-references,memory64` and run as
  components with Wasmtime; the fallback artifacts were validated with
  `wasm-tools validate` and run as core modules.
- The focused quality checks and `git diff --check` passed. The requested
  `verify lane --gate cli-parity` reached the quality gate successfully but
  the unrelated CLI parity gate remains red because the pinned bootstrap
  `--help` golden and an existing `stdio::println` lint fixture are already
  out of sync. This receipt is partial evidence only; the acceptance boxes,
  negative/diagnostic coverage, and final close review remain open.
- This is progress, not a close receipt. Complete tag/payload observable
  semantics, cross-backend runtime parity, and the final issue-close review
  remain open; no acceptance checkbox is marked complete from this partial
  evidence.

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
