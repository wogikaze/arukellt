---
Status: done
Created: 2026-09-19
Updated: 2026-09-19
ID: 858
Parent: 852
Track: language-v2
Depends on: "856"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 14"
---

# 858 — 演算子 Trait へ統一し magic method を退役する

## Scope

RFC-011: **14**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] 各演算子を canonical Trait method に対応づける
- [x] 演算子解決を Trait coherence に従わせる
- [x] 旧 magic method の移行方針を定める
- [x] builtin/user-defined/generic fixture を追加する
- [x] stdlib/compiler の二重実装を残さない

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Evidence

- Implementation commit: `31c76ce33` (`feat: route operators through canonical traits`).
- Normative decisions: [ADR-065](../../docs/adr/ADR-065-language-v2-operator-trait-resolution.md)
  adopts the canonical mapping and supersedes [ADR-038](../../docs/adr/ADR-038-operator-overload-traits.md);
  `docs/language/spec.md` and RFC-011 are synchronized.
- Canonical mapping and coherence: `src/compiler/typechecker/operator_traits.ark`
  maps arithmetic, bitwise, unary, equality, ordering, and index syntax to the
  canonical Trait names. Concrete receivers use the existing Trait registry and
  coherence lookup; generic receivers require the matching function bound.
  `src/compiler/mir/lower/binary_method.ark` and the lowerers use the same
  canonical method names. Scalar opcodes remain only the equivalent backend fast path.
- Migration boundary: legacy `__add`, `__sub`, `__mul`, `__div`, `__rem`,
  `__eq`, and `__cmp` are excluded from candidate resolution. The diagnostic
  fixture `tests/fixtures/diagnostics/operator_trait_missing.ark` proves that an
  inherent `add` method does not satisfy `Add`; it reports E0208.
- Positive fixtures: `tests/fixtures/trait/operator_overload.ark` covers a
  user-defined type, equality/inequality, and ordering;
  `tests/fixtures/stdlib_trait/operator_generic.ark` covers a generic `T: Add`
  call with both a user-defined `Point` and builtin `i32`.
- Target parity: the GC component and linear-memory modules both validated and
  executed the operator and generic fixtures. Both produced `4, 6, 2, 2, 1, 1, 1`
  for the operator fixture and `4, 6, 42` for the generic fixture.
- Verification: targeted Ark formatting passed (`checked=16 failed=0`),
  `verify lane --gate quality-quick` passed (changed-file lint and ratchet passed),
  `docs check` passed 4/4 with the current artifact, and `check-adrs.py` passed.
- Canonical `selfhost build-compiler` remains blocked by the pre-existing #854
  generic-enum return mismatch in `unbox` and `slice_get`; the current artifact
  used for the receipts is a targeted bootstrap artifact, not a canonical rebuild claim.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
