---
Status: open
Created: 2026-09-19
Updated: 2026-09-20
ID: 874
Parent: 852
Track: language-v2
Depends on: "854, 855, 859, 860, 873"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 8 (fallback lowering)"
---

# 874 — fallback managed-value lowering を実装する

## Scope

RFC-011: **8 (fallback lowering)**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] struct/enum/Option/Result の fallback layout を実装する
- [ ] closure env を fallback heap へ lower する
- [ ] Vec/String/HashMap を fallback runtime に接続する
- [ ] 共有・循環参照を reject せず実行する
- [ ] Wasm GC backend と同じ move/borrow semantics を保つ

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する

## Progress receipt — 2026-09-20 (aggregate allocation boundary)

- Implementation commit `6f480e71e` connects linear-memory `struct` and array
  constructors, plus the fallback `Vec` layout constructor, to the private
  `LINEAR_GC_ALLOC` runtime function. The allocator records stable payload
  ranges and can reuse swept records; the old raw heap bump remains available
  for migration paths that have not yet supplied managed descriptors.
- Normative contract: [ADR-075](../../docs/adr/ADR-075-language-v2-linear-memory-tracing-runtime.md)
  and the [linear-memory lowering plan](../../docs/plans/wasm32-linear-memory-lowering.md),
  with RFC-011 section 8 as the language-v2 scope. No new public API or ABI
  was introduced.
- Positive fixtures and outputs: `tests/fixtures/enums/enum_struct_variant.ark`
  produces `75\n24` on both backends; `tests/fixtures/collections/vec_option_i32.ark`
  produces `7` on the fallback; `vec_string_println_get.ark` produces `hi`,
  and `generics/gc_struct_field_vec_string.ark` produces `1` on the fallback.
  The GC enum fixture was packaged as a component; the linear artifacts were
  validated as core modules and contain no `struct.*`, `array.*`, or `ref.*`
  instructions.
- Diagnostic coverage: `tests/fixtures/diagnostics/non_exhaustive.ark` still
  rejects with `E0200` and missing `Color::Blue`; allocator wiring does not
  bypass enum diagnostics.
- Verification receipts:

  ```text
  python3 scripts/manager.py selfhost build-compiler       PASS
  python3 scripts/manager.py verify lane                  PASS
  python3 scripts/tests/test_linear_gc_runtime_contract.py PASS (4 tests)
  python3 scripts/tests/test_linear_gc_runtime_e2e.py      PASS (1 test)
  ```

- Representative artifact hashes (SHA-256):

  ```text
  wasm32-gc enum component 3159 bytes  be00f6c48b931eb26c732701fad6cb00664c6ae14ad2f1a1994877be9255b914
  wasm32 enum core         2489 bytes  f7509d77104d18c4f7f547c9f68630b0412401dbb050bb71d74cf8bac6c28597
  wasm32 Vec<Option<i32>> 2269 bytes  87f29da04f696d0834d4483e2e20ae998d683eb3d1ce2f1a04daf83b6cd889b7
  ```

- Tool versions were `wasmtime 46.0.1 (823d1b8f2 2026-06-24)` and
  `wasm-tools 1.252.0`. Root-frame insertion and automatic collection remain
  disabled until all managed locals have descriptor-aware updates. String and
  HashMap descriptor lowering, closure environments, shared/cyclic source
  graphs, and the complete move/borrow parity audit remain open; this receipt
  does not close #874.
