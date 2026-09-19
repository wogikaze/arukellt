---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 867
Parent: 852
Track: language-v2
Depends on: "866"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 11, 31, 38"
---

# 867 — `pub` / Component export / WIT import 境界を分離する

## Scope

RFC-011: **11, 31, 38**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] `pub` を source visibility に限定する
- [ ] Component export/import を別契約にする
- [ ] WIT package identifier の canonical path を統一する
- [ ] name mangling 具体形式を language spec から外す
- [ ] source namespace と WIT namespace の分離 fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Progress receipt (not done; 2026-09-19)

- Implementation: `d1f3014d9` (`language-v2: separate source and component visibility`).
- Normative updates: `docs/adr/ADR-071-component-export-visibility-boundary.md`,
  `docs/language/spec.md`, `docs/spec/import-system.md`, and RFC-011 references.
- Frontend/compiler changes: `export` is a separate function declaration fact,
  AST cache serialization preserves it, WIT package/interface identities use a
  shared full-semver validator, and source visibility no longer implies a
  Component export. Existing Component fixtures were migrated from `pub fn` to
  `export fn`; positive, negative, diagnostic, and namespace-separation fixtures
  were added.
- Verification: `verify lane --gate component-interop` reached `quality changed`
  PASS (lint 247, ratchet PASS). Targeted formatting, comment policy, semantic
  debt, docs regeneration, and script unit tests pass. The component-interop lane
  fails 102 runtime cases because the current bootstrap artifact still reports
  the pre-existing #854 `unbox`/`slice_get` generic return mismatch.
- The canonical `selfhost build-compiler` has the same #854 blocker, so current
  source compile/validate/runtime evidence for this issue is not yet valid.
  Acceptance remains unchecked and this issue stays open.

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
