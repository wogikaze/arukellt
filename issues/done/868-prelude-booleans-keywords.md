---
Status: done
Created: 2026-09-19
Updated: 2026-09-20
ID: 868
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 32, 33, 41"
---

# 868 — Prelude / boolean literal / keyword policy を固定する

## Scope

RFC-011: **32, 33, 41**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] Prelude を小さく stable な固定集合として列挙する
- [x] `true`/`false` を lexical literal に一意化する
- [x] hard/contextual/future-reserved を分類する
- [x] 未実装を理由に future-reserved を解放しない
- [x] parser/formatter/highlighting fixture を同期する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Progress receipt (not done; 2026-09-19)

- Implementation: `c19a95bb1` (`language-v2: fix prelude and keyword policy`).
- Normative updates: [ADR-072](../../docs/adr/ADR-072-language-v2-prelude-and-keyword-policy.md),
  RFC-011 items 32, 33, 41, `docs/language/spec.md`, and
  `docs/data/language-keyword-policy.toml`.
- Prelude: `std/manifest.toml` now records the v2 stable set separately from
  the legacy broad auto-import boundary; `true`/`false` value entries were
  removed and remain lexical `TK_BOOL` literals. The legacy boundary remains
  explicit until #878.
- Keyword implementation: hard keywords and boolean literals use the existing
  keyword token path; `test`, `move`, `dyn`, `type`, and `mod` remain contextual;
  `await`, `unsafe`, `extern`, and `super` produce `TK_RESERVED`. LSP rename,
  playground tokenization/highlighting, formatter fixture, and diagnostic
  fixtures use the same classification.
- Verification: playground TypeScript build and all 182 tests pass; `quality
  changed` passes (fmt 7/7, lint 7/7, ratchet PASS); comment policy,
  semantic-debt, stdlib manifest, docs consistency, ADR ledger, and Python
  docs/gate tests pass. `docs regenerate` passes in the current working tree.
- Historical blocker (2026-09-19): the pinned bootstrap rejected the generic
  enum payload path before current-source evidence could be collected. It is
  superseded for this issue by the guarded bootstrap overlay in `34f855148`.

## Evidence (current-source recheck; 2026-09-20)

- `34f855148` keeps the copied WIT closure's type identities stable in the
  isolated bootstrap overlay. `python3 scripts/manager.py selfhost
  build-compiler` passes, and `python3 scripts/manager.py verify lane` passes
  for the changed compiler/verification surface.
- `python3 scripts/manager.py selfhost diag-parity` reports `PASS=54
  SKIP=26 FAIL=1`; both `diagnostics/boolean_literal_not_name.ark` and
  `diagnostics/future_reserved_keyword.ark` pass. The single failure is the
  unrelated `diagnostics/capability_stub.ark` warning-column golden.
- The current s2 compiler returns the expected parse diagnostics for both
  boolean-as-name and future-reserved-name fixtures. The current formatter
  produces the committed `fmt/keyword_policy.expected` output, and the
  playground build plus test suite passes `182/182`, including lexical
  highlighting and keyword-policy tests.
- Acceptance is complete for this issue. The unrelated diagnostic golden is
  retained as an open verification follow-up; it is not hidden or skipped by
  this issue.

## Historical progress receipt (superseded; 2026-09-20)

- Parser recovery fix: `29ad8cd46` (`fix: advance parser after rejected token`).
  `expect_token` now consumes a non-EOF token after recording a mismatch, so a
  reserved or boolean token used where an identifier is required cannot be
  revisited forever by an enclosing parameter loop.
- Current s2 verification: `future_reserved_keyword.ark` and
  `boolean_literal_not_name.ark` both terminate with the expected
  `error[E0001|parse]` instead of trapping with an out-of-bounds memory access;
  `parse_bad_token.ark` retains its two parse diagnostics.
- Related grammar smoke checks: `fn_multiclause_semicolon_boundary.ark`
  reports its expected duplicate-definition diagnostic, and
  `fn_multiclause_where_semicolon.ark` exits successfully with warnings only.
- At the time of this receipt, `selfhost diag-parity` had improved to
  `PASS=52 SKIP=26 FAIL=1`. The remaining
  failure is the unrelated `diagnostics/capability_stub.ark` golden, whose
  expected source column is 4 while current output reports column 8. The
  formatter parity gate still has the pre-existing interleaved/unsorted import
  fixture failures. The later current-source recheck above supersedes this
  provisional status.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
