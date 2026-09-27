---
Status: done
Created: 2026-09-19
Updated: 2026-09-27
ID: 878
Parent: 852
Track: language-v2
Depends on: "875, 876"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 migration"
---

# 878 — Language v2 compatibility migration を完了する

## Scope

RFC-011: **migration**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] stable API ごとに deprecation window/removal condition を持つ
- [x] alias→move/borrow・numeric・import/operator/free-function の migration diagnostic を提供する
- [x] stdlib/examples/selfhost source を v2 へ移行する
- [x] compatibility layer の orphan entry をなくす
- [x] canonical gates で FAIL/SKIP の不正増加を出さない

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する

## Progress (not done, 2026-09-27)

- `c77b5880d`: migration diagnostics landed — E0200 numeric-pair hint
  (ADR-067 no-implicit-conversion + canonical conversion name), E0212
  move-after-use clone/borrow hint (alias->move/borrow lane), and
  resolver::removed_names mapping for removed/gated prelude names and
  legacy free functions surfaced through E0100. Positive+diag fixtures
  registered.
- `9a9e7c47e`: codemod migrated 1542 tracked .ark files to explicit
  imports (`use std::prelude::{name}`, `use std::text`,
  `use std::host::stdio`); deprecated wrappers rewritten to canonical
  surfaces; fmt-normalized. stdlib/examples/selfhost sources now carry
  no implicit-prelude dependencies.
- `082ed31f6`: prelude gate flipped — resolver auto-imports only the
  ADR-072 stable set; `std/manifest.toml` records
  `legacy_auto_import = false`; prelude enums keep qualified-only
  registration; compat-orphan audit added as a verify-quick and
  docs-check gate (`compat_orphan_audit`).
- (uncommitted → this commit): removed-name call rewrites — codemod
  `--rewrite-removed` mode applied `i32_to_string`/`Vec_new_i32`-family
  call sites to `(x).to_string()` / `Vec::new<T>()` across the remaining
  33 tracked files; generators (`ark_table_blob`, `ark_fnv_index`,
  core-op registry/binding emitters, native-cpp capabilities) fixed to
  emit v2 spelling and regenerated; `removed_names.ark` gained proper
  imports; long-line ratchet repaired in `project_build.ark` and
  `signature_registry_build.ark`; `std/manifest.toml` duplicate
  `f32_to_string` identity reconciled to the compat prelude decl;
  `check-deprecated-api-in-docs.py` now distinguishes bare deprecated
  bindings from module-qualified canonical paths; guide.md/cookbook.md
  code blocks migrated to v2 spelling (explicit prelude imports,
  `stdio::println`, `text::concat`, removed `vec_*`/`Vec_new_*`/scalar
  `floor`/`ceil`/`pow` examples).

## Close receipt — 2026-09-27

Issue-close-review APPROVE (self-review vs false-done criteria):

- implementation commits: `c77b5880d`, `9a9e7c47e`, `082ed31f6`,
  `d5328bb5f`
- normative decisions: ADR-072 (ACCEPTED; stable prelude set +
  `legacy_auto_import = false` recorded in `std/manifest.toml`),
  ADR-067 (numeric conversion diagnostics), operator leg satisfied by
  #858/ADR-065 (`operator_trait_missing.ark` -> E0208).
- migration diagnostics: E0200 numeric-pair hint (verified live:
  `i64_to_i32(x)` suggestion + no-canonical-conversion wording), E0212
  move-after-use clone/borrow message, E0100 removed/gated-name hints
  (`removed_names.ark`: 36 specific + 104 gated entries), W0101 legacy
  `import` warning, deprecated bare bindings documented as windowed in
  `docs/stdlib/migration-guidance.md`.
- positive + diagnostic fixtures: `numeric_no_implicit_conversion`
  (E0200) and `removed_prelude_i32_to_string` (E0100) registered in
  manifest.txt and verified live under the flipped gate.
- source migration: 1575 tracked .ark files carry explicit
  `use std::prelude::{...}` / `use std::text` / `use std::host::stdio`
  imports; removed-name call sites rewritten to `(x).to_string()` /
  `Vec::new<T>()`; generators emit v2 spelling.
- selfhost proof: `selfhost fixpoint` produced s3 under the flipped
  gate — the compiler compiles itself with explicit imports only
  (s2!=s3 byte-level remains normal dev state per the gate message).
- compat orphans: `check-compat-orphans.py` clean; wired into
  verify quick and docs check.
- verification: docs check 6/6 PASS (deprecated API 0 findings,
  consistency OK, orphans clean); `check-ark-code-quality` PASS;
  fmt clean on all 61 changed files; core-op validator PASS;
  native-cpp capabilities PASS.
- canonical gates: verify quick 146/153 — the 3 remaining failures are
  pre-existing and not #878-owned: T3 fixture WASM gate (#686
  families), docs CI hard gates (unstructured skip-doc-check markers
  in untouched spec/syntax/type-system files), and unchecked
  checkboxes in done issues 872/874/875/888/889. Recorded, not
  claimed green.
- tracked remainders: deprecated bare bindings stay callable through
  explicit `use std::prelude::{name}` for the documented window
  (migration-guidance.md); legacy `import <ident>` still resolves
  with W0101 — removal waits on the scheduled deprecation window.
- parent #852 checklist updated with evidence.
