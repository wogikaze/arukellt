---
Status: done
Created: 2026-09-19
Updated: 2026-09-19
ID: 856
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, ADR-062, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 12, 17"
---

# 856 — Trait core / coherence / generic arity を確定する

## Scope

RFC-011: **12, 17**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] `Self`、型引数、associated type/function を実装する
- [x] coherence/conflicting impl を診断する
- [x] blanket impl の適用条件と termination を固定する
- [x] static dispatch と `dyn Trait` の境界を固定する
- [x] generic parameter count に恣意的な小さい仕様上限を残さない

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Evidence

- Implementation commit: `782c34b2980368400a5a076ea72f20b369234bfd`.
- Normative decision: [ADR-062](../../docs/adr/ADR-062-language-v2-trait-core-coherence.md), with synchronized updates to `docs/language/spec.md`, RFC-004, RFC-011, and the diagnostic catalog.
- Positive fixtures: `tests/fixtures/trait/trait_core_v2.ark` covers `Self`, trait type arguments, associated type/function, projection, and a generic bound method; `tests/fixtures/generics_v1/generic_three_param.ark` proves no semantic three-parameter cap.
- Diagnostic fixtures: `tests/fixtures/diagnostics/trait_blanket_impl.ark` (E0216), `trait_dyn_boundary.ark` (E0217), `trait_wrong_arity.ark` (E0218), and the existing overlapping-impl fixture (E0215).
- Current-artifact build receipt: temporary current selfhost artifact `.build/bootstrap-shim-856-final/selfhost/arukellt-s2.wasm`; registry/core-op audits reported `mismatched=0`, `unresolved=0`.
- Target parity: `trait_core_v2` compiled and validated as `wasm32-gc` component and `wasm32` linear-memory fallback; both ran with output `79`. The three-parameter fixture compiled and validated on both paths; both ran with output `7`.
- Quality/docs gates: `verify lane --gate quality-quick` PASS; `quality structure` PASS; `docs check` PASS (4/4); comment policy, semantic-debt, formatter, lint, and lint ratchet PASS.
- Fixture gate: current-only `PASS=1695 FAIL=1 COMPILE_VALIDATE=1696 EXECUTED=101 RUNTIME_UNSUPPORTED=2`. The sole failure, `match_extensions/struct_pattern.ark`, reproduces with the pinned bootstrap and the pre-existing s2 artifact as `unknown enum 'Point`; it is an unrelated pre-existing selfhost exhaustiveness gap and is not claimed as fixed here.
- Canonical `selfhost build-compiler`: still blocked by the pre-existing #854 generic-enum bootstrap mismatch (`unbox` and `slice_get` return-type diagnostics). The temporary artifact is a targeted receipt only and is not a canonical bootstrap success claim.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
