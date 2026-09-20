---
Status: done
Created: 2026-09-19
Updated: 2026-09-20
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

- [x] multi-clause grouping を adjacency に依存させない
- [x] pattern-bound variable と `where` scope を固定する
- [x] newline/semicolon/continuation grammar を固定する
- [x] ambiguous parse の negative fixture を追加する
- [x] parser/formatter round-trip を一致させる

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
- Historical blocker (2026-09-19): the pinned bootstrap rejected the generic
  enum payload path before current-source evidence could be collected. It is
  superseded for this issue by the guarded bootstrap overlay in `34f855148`.

## Evidence (current-source recheck; 2026-09-20)

- `34f855148` keeps the copied WIT closure's type identities stable in the
  isolated bootstrap overlay. `python3 scripts/manager.py selfhost
  build-compiler` passes, and changed-source `verify lane` passes.
- With `ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2-runtime.wasm`,
  `fn_multiclause_where_semicolon.ark` compiles to a standard WASI P2
  component, `wasm-tools validate` passes, and Wasmtime prints the committed
  `high` / `low` output. The semicolon-boundary fixture returns the expected
  duplicate-definition diagnostic.
- The current s2 formatter produces the committed
  `fmt/multiclause_where.expected` output and remains idempotent. The
  current-only selfhost fixture gate still reports six unrelated legacy
  selfhost fixture failures; this issue's positive and negative fixtures pass
  independently and the failures are not hidden or allow-listed here.
- Acceptance is complete for this issue. The remaining broad fixture backlog
  is tracked outside the multi-clause grammar scope.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
