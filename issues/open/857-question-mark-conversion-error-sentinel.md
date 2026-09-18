---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 857
Parent: 852
Track: language-v2
Depends on: "694, 856"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 13, 19"
---

# 857 — `?` / conversion / Error sentinel を整理する

## Scope

RFC-011: **13, 19**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] From/Into/TryFrom/TryInto の関係を固定する
- [x] `?` を Option/Result と異種 error conversion に統合する
- [x] 変換不能 diagnostic を追加する
- [x] compiler-internal Error sentinel を user type system から分離する
- [x] 関連 fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Evidence

- Implementation: `d6ce3f5d4ee1e8ca82fdbff7292579a30e7edaf3`.
- Normative decision: [ADR-063](../../docs/adr/ADR-063-language-v2-conversion-error-boundary.md),
  synchronized with the language spec, error-handling guide, compiler IR/error-code
  catalog, and RFC-011.
- The typechecker now validates `?` before CoreHIR/MIR lowering. `Result` accepts
  identity propagation or exactly one `From<source> for target`; `Option` remains
  `Option`-only, and Option/Result mixing, missing conversion, and ambiguous
  conversion are rejected with E0210. `TryFrom`/`TryInto` are explicitly not
  implicit `?` candidates; their future definition remains #708's scope.
- Positive fixture: `tests/fixtures/question_mark/user_error_type.ark` proves a
  user-defined `enum Error` plus `From<String> for Error` propagation. Negative
  fixture: `tests/fixtures/diagnostics/question_from_missing.ark` proves the
  missing-conversion diagnostic.
- Current-artifact receipt: `.build/bootstrap-shim-857/selfhost/arukellt-s2.wasm`
  compiled the new typechecker with registry/core-op audits at
  `mismatched=0` / `unresolved=0`.
- Target parity: the positive fixture compiled and validated as both a
  `wasm32-gc` component and a `wasm32` linear-memory module; both executed with
  output `OK`. `verify lane --gate quality-quick` and `verify lane --gate
  diag-parity` passed; `docs check` passed 4/4 with the current artifact.
- Current-only fixture gate: `PASS=1696 FAIL=1 COMPILE_VALIDATE=1697
  EXECUTED=101 RUNTIME_UNSUPPORTED=2`. The only failure is the pre-existing
  `match_extensions/struct_pattern.ark` unknown-enum diagnostic reproduced by
  the #856 baseline; it is outside #857.
- Canonical `selfhost build-compiler` remains blocked by the pre-existing #854
  generic-enum bootstrap mismatch in `unbox` and `slice_get`. The current
  artifact used above is a targeted receipt, not a canonical bootstrap success.

## Remaining blocker

The implementation-specific acceptance conditions are complete, but this issue
stays open because its declared dependency #694 (`Error` trait and the unified
stdlib error ecosystem) is still open. Do not move #857 to `issues/done/` until
#694 is resolved and the applicable integrated verification has been rerun.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
