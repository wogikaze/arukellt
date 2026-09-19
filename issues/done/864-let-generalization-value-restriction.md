---
Status: done
Created: 2026-09-19
Updated: 2026-09-19
ID: 864
Parent: 852
Track: language-v2
Depends on: "856"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 27"
---

# 864 — let-generalization / value restriction を形式化する

## Scope

RFC-011: **27**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] generalize 可能な non-expansive expression を定義する
- [x] allocation/call/mutable state の扱いを固定する
- [x] polymorphic let の positive/negative fixture を追加する
- [x] 型推論を backend 非依存にする
- [x] 将来 effect system との置換境界を文書化する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Implementation receipt

- Implementation: `f9393be42` (`feat: enforce let generalization value restriction`).
- Normative decision: [ADR-070](../../docs/adr/ADR-070-language-v2-let-generalization.md).
  The rule is reflected in [language spec §2.8](../../docs/language/spec.md#28-let-generalization-typescheme),
  the [type-system guide](../../docs/language/type-system.md), and RFC-011 item 27.
- The common typechecker now generalizes only immutable `let` bindings whose
  initializer is a literal, closure, or already-generalized immutable alias.
  Calls, aggregates, blocks, mutable bindings, and environment-owned type
  variables remain monomorphic. Local function calls now unify argument types,
  so the negative fixtures exercise the intended restriction rather than a
  diagnostic gap in the call path.
- Positive fixtures: `typescheme_value_restriction.ark` and the existing
  `typescheme_poly_let.ark`. Negative fixtures:
  `typescheme_mutable_binding.ark`, `typescheme_expansive_call.ark`, and
  `typescheme_environment_monomorphic.ark`, each with a committed
  `.selfhost.diag` golden.
- `python3 scripts/manager.py verify lane`: PASS after the quality gate was
  taught to classify manifest negative fixtures outside the legacy
  `tests/fixtures/diagnostics/` directory. The focused unit tests for this
  classification also pass, and `selfhost diag-parity` reports PASS with
  zero skips.
- Current-source targeted artifact receipts: the positive fixture compiled
  and validated for `wasm32-gc` and `wasm32`, its component validated, and both
  runtimes printed the same output: `42` followed by `value restriction`.
  The typecheck result is shared before backend lowering; no backend-specific
  TypeScheme path was added.
- `docs regenerate` and docs freshness/consistency checks pass. The docs
  example subcheck still reports the pre-existing `ADR-066` i64/i32 example
  failure (4 pass, 1 fail, 283 skip); that block is outside this issue and is
  not reclassified here. Full current fixture parity remains `PASS=1530,
  FAIL=176`; its failures are existing global i64/i32, Ordering, closure,
  HashMap, and runtime/validation backlog entries, not these fixtures.
- The canonical `selfhost build-compiler` remains blocked by the pre-existing
  #854 `unbox`/`slice_get` bootstrap mismatch. The targeted current-source
  artifact used above is recorded as such and is not claimed as a canonical
  bootstrap or fixpoint receipt. Backend closure-capture/alias lowering remains
  owned by #872; this issue's GC runtime receipt intentionally uses the direct
  non-expansive closure form.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
