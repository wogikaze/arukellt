---
Status: done
Created: 2026-09-19
Updated: 2026-09-20
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

- [x] `pub` を source visibility に限定する
- [x] Component export/import を別契約にする
- [x] WIT package identifier の canonical path を統一する
- [x] name mangling 具体形式を language spec から外す
- [x] source namespace と WIT namespace の分離 fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Close receipt (done; 2026-09-20)

- Implementation: `d1f3014d9` (`language-v2: separate source and component visibility`).
- Normative updates: `docs/adr/ADR-071-component-export-visibility-boundary.md`,
  `docs/language/spec.md`, `docs/spec/import-system.md`, and RFC-011 references.
- Frontend/compiler changes: `export` is a separate function declaration fact,
  AST cache serialization preserves it, WIT package/interface identities use a
  shared full-semver validator, and source visibility no longer implies a
  Component export. Existing Component fixtures were migrated from `pub fn` to
  `export fn`; positive, negative, diagnostic, and namespace-separation fixtures
  were added.
- Current-source stage-2 evidence uses `.build/selfhost/arukellt-s2.wasm`; the
  historical pinned-bootstrap `#854` mismatch is superseded for this receipt by
  the guarded source-only overlay in `34f855148`.
- Positive boundary fixture: `tests/fixtures/component/export_visibility_boundary.ark`
  emits only `component-only` and `public-and-component`; `source-only` is absent
  from the WIT and packaged Component export surface. `wasm-tools validate` passes,
  and Wasmtime Component Model invocations return `42` for both exports.
- Negative/diagnostic fixtures: `component/export_non_function.ark` returns
  `error[E0001|parse]`, and `modules/export_not_source_public/main.ark` returns
  `error[E0100|resolve]` for access to the `export`-only source-private function.
- WIT identity and namespace evidence: `wit_import/namespace_separation/main.ark`
  with `host_math.wit` compiles and validates; the Component contains the canonical
  `test:host/math@0.1.0` import and the separate `add` export. The source-level
  `import "test:host/math@0.1.0"` path is therefore not interpreted as a `use`
  module path. `gate-654` and `gate-670` both pass.
- GC/fallback boundary evidence: the positive fixture passes official P2
  component packaging on `wasm32-gc` and linear `wasm32/wasi-p1` core
  compile/validate/runtime. `gate-666-component-library-emit.py` also passes the
  library WIT golden and official P2 packaging gate.
- Verification receipts: `python3 scripts/check/gate-654-wit-import-component-emit.py`
  PASS; `python3 scripts/check/gate-670-wit-import-resolver-hardening.py` PASS;
  `python3 scripts/check/gate-666-component-library-emit.py` PASS; targeted current
  s2 fixture compile/diagnostic/validation/runtime commands PASS; `docs regenerate`
  reports generated docs up to date and `docs check` is 4/4 PASS after this issue
  move.
- Close review: APPROVE. Dependency #866 is done; no acceptance item is hidden by
  a SKIP or allow-list. `wasm32 + wasi-p2 + component` is intentionally rejected
  by the accepted target capability matrix, not an unverified fallback path.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
