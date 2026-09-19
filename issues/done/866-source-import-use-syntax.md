---
Status: done
Created: 2026-09-19
Updated: 2026-09-19
ID: 866
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 30"
---

# 866 — source module の import / use 構文を整理する

## Scope

RFC-011: **30**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] `import` と `use` の役割を重複させない
- [x] alias/selective import/re-export を仕様化する
- [x] name resolution/formatter/LSP を同期する
- [x] 旧構文 migration diagnostic を追加する
- [x] WIT 境界は #867 に分離する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Implementation receipt

- Implementation: `963d544a6` (`feat: define import and use module boundaries`).
- Normative sources: [source import system](../../docs/spec/import-system.md),
  [language spec §7.1 and Appendix A](../../docs/language/spec.md#71-import-syntax),
  [syntax guide](../../docs/language/syntax.md), and the RFC-011 item 30 update.
  Existing accepted ADR-009/ADR-031 boundaries remain in force; no new ADR was
  required. `use` is the canonical source-module form, legacy local `import`
  is migration-only, and string-literal/WIT imports remain owned by #867.
- The parser, resolver, loader, formatter, and LSP now share module alias and
  selective-use behavior. `pub use` re-exports only public source symbols;
  private symbols remain hidden. The formatter handles both the canonical and
  migration forms.
- Positive coverage: `module_alias/main.ark` (legacy alias compatibility),
  `module_import/use_func_destructure.ark` (selective `use`), and
  `pub_use_reexport_visible/main.ark` (public re-export) all selfhost-check
  successfully. `pub_use_nonpub_hidden/main.ark` fails as expected with
  `E0100|resolve: undefined module member: facade::answer`.
- Migration coverage: `diagnostics/deprecated_import_syntax.ark` and its
  dependency selfhost-check successfully and emit the exact committed
  `W0101|parse` diagnostic. The fixture is registered in
  `docs/data/warnings.toml` and `tests/fixtures/manifest.txt`.
- Targeted current-source artifact receipt (SHA-256
  `29436204b9abaeca69c9f2242a09b13b8071abd1c33baf9a480015dad210c1c8`):
  `wasm32-gc` component compile/validate passed for the alias and re-export
  fixtures, and the `wasm32` linear-memory compile/validate passed for the
  alias fixture. Wasmtime produced identical alias output `hello, world` on
  GC and linear paths; the re-export path produced `7` on GC. The GC audits
  reported all candidates matched (reg-vt 1/1, core-op shadow 5/5 for the
  alias fixture), and the linear audits passed.
- Verification: `python3 scripts/manager.py verify lane` PASS;
  `docs regenerate` PASS; docs freshness/deprecated-API/consistency checks
  PASS. Targeted diagnostic parity is `PASS=49 SKIP=26 FAIL=0`. The broad
  current-only fixture probe observed `PASS=1441 FAIL=265` across
  `COMPILE_VALIDATE=1706`, with `EXECUTED=95` and `RUNTIME_UNSUPPORTED=2`;
  its existing global backlog is not reclassified as this issue's failure.
  The docs example subcheck retains the known #854 generic return mismatch
  (`0 pass, 6 fail, 282 skip`); the three failing blocks are unrelated to
  import/use semantics.
- The required `ARUKELLT_CC=clang python3 scripts/manager.py verify quick`
  rerun completed with `145/151 PASS, 6 FAIL`. The six failures are outside
  this issue: two pre-existing unregistered fixtures, #663/#664 failing on
  the #854 generic return mismatch, #798 core-op registry/validator drift,
  #686 T3 fixture backlog, and the repository-wide Ark thin-wrapper ratchet.
  The #866-specific docs hard gate passes after the example was kept inline,
  so no new skip budget or allow-list entry remains.
- The canonical pinned `selfhost build-compiler` and fixpoint remain blocked by
  the pre-existing #854 `unbox`/`slice_get` generic return mismatch. The
  targeted artifact above is explicitly not claimed as a canonical bootstrap
  or fixpoint receipt. WIT/component-boundary implementation is deferred to
  #867 as required by this issue.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
