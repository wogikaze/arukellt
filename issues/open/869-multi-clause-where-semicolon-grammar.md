---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 869
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, ADR-073, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 34–36"
---

# 869 — multi-clause / where / semicolon grammar を確定する

## Scope

RFC-011: **34–36**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] multi-clause grouping を adjacency に依存させない
- [ ] pattern-bound variable と `where` scope を固定する
- [ ] newline/semicolon/continuation grammar を固定する
- [ ] ambiguous parse の negative fixture を追加する
- [ ] parser/formatter round-trip を一致させる

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Progress receipt (not done; 2026-09-19)

- Implementation: `203029f18` (`language-v2: define multiclause grammar boundaries`).
- Normative updates: [ADR-073](../../docs/adr/ADR-073-language-v2-multiclause-where-semicolon-grammar.md),
  RFC-011 items 34–36, `docs/language/spec.md`, and
  `docs/language/syntax.md`.
- Grammar: clause groups continue only across newline tokens; an explicit
  semicolon terminates the group. `where` bindings are ordered, may use the
  synthetic function parameter and earlier bindings, and are separated by
  newlines or semicolons. Clause guards/bodies retain their own pattern scope.
- Fixtures: `selfhost/fn_multiclause_where_semicolon.ark` covers operator
  continuation plus a semicolon-terminated `where` binding; the
  `selfhost/fn_multiclause_semicolon_boundary` diagnostic fixture covers the
  duplicate-definition boundary; `fmt/multiclause_where.ark` covers formatter
  idempotency and parse acceptance. Manifest count and generated ADR index are
  synchronized.
- Verification: targeted `fmt --check` and `quality changed` pass (fmt 11/11,
  lint 11/11, ratchets PASS); comment policy, semantic-debt, ADR ledger,
  docs freshness/consistency, and Python docs/gate tests pass. `docs regenerate`
  passes in the working tree.
- Blocker: `python3 scripts/manager.py selfhost build-compiler` and the
  selfhost formatter-parity lane remain blocked by the pre-existing #854
  bootstrap mismatch (`unbox` and `slice_get` declared and inferred as the
  same generic `T`). The stale artifact accepts the new positive fixture, but
  it cannot validate the new semicolon-boundary diagnostic; this is not
  current-source evidence. Canonical compile/validate/runtime acceptance is
  therefore still unchecked and #869 stays open.

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
