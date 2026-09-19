---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
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

- [ ] Prelude を小さく stable な固定集合として列挙する
- [ ] `true`/`false` を lexical literal に一意化する
- [ ] hard/contextual/future-reserved を分類する
- [ ] 未実装を理由に future-reserved を解放しない
- [ ] parser/formatter/highlighting fixture を同期する

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
- Blocker: `python3 scripts/manager.py selfhost build-compiler` still fails on
  the pre-existing #854 bootstrap mismatch (`unbox` and `slice_get` declared
  and inferred as the same generic `T`). The current artifact therefore cannot
  provide valid current-source compile/validate/runtime evidence for this
  issue. `docs check` has the same three known doc-example failures. Acceptance
  remains unchecked and #868 stays open.

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
