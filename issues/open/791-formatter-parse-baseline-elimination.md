---
Status: open
Created: 2026-07-13
Updated: 2026-07-13
ID: 791
Track: tooling-contract
Depends on: "785"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ-06 formatter full-repository audit
---

# 791 — Eliminate the Ark canonical parser baseline

## Summary

Remove every content-addressed fmt/lint parser exception from
`docs/data/ark-formatter-baseline.toml`. The current set contains compiler and
stdlib sources that the formatter parse gate rejects because of legacy source
forms, parser gaps, or source corruption. The baseline is a temporary migration
boundary, not permission to add more files.

## Acceptance

- [ ] Every listed file passes `python3 scripts/manager.py fmt --check <path>`
- [ ] Every listed compiler/stdlib file passes `python3 scripts/manager.py lint <path>`
- [ ] Every repaired file still compiles through the selfhost bootstrap path
- [ ] `docs/data/ark-formatter-baseline.toml` has zero exceptions and is removed
- [ ] `python3 scripts/manager.py quality full` passes without formatter skips

## Re-evaluation

Owner: compiler-tooling. Re-evaluate by 2026-08-31. An exception may be removed
only after format output reparses and the relevant compiler/stdlib verification
passes.

## Progress receipt — 2026-09-25 (baseline eliminated)

Commit `3377a3712` removed all 11 remaining exceptions and deleted
`docs/data/ark-formatter-baseline.toml`.

Repairs by root cause:
- Raw newlines inside string literals (canonical lexer rejects them as
  unterminated): `wasm/wat.ark`, `wasm/wat_function_body.ark`,
  `wasm/wat_functions.ark` rewritten with `\\n` escapes; emitted WAT bytes
  unchanged.
- Keyword `if` used as a module name in `use mir::lower::if` (unparseable):
  file renamed to `if_ops.ark`, importers `body_if.ark` / `core_if.ark`
  updated.
- Source corruption: `main/manifest_doc_site.ark` `doc_site_module_link` was
  missing one closing paren since its introduction (d90c463bc); the file is
  orphan since 5a30f4227 unwired `doc --html` (shell-script implementation
  remains the live path). Paren balanced, trailing commas removed.
- Trailing commas in call argument lists: `contract_allows_tuple.ark`,
  `manifest_doc_site.ark`.
- Already-parseable entries removed as-is: `mir/lower/body_match.ark`,
  `typechecker/returns.ark`, `std/toml.ark`, `std/wit/printer.ark`,
  `diagnostics/codes.ark` (removed earlier on this branch).

Tooling: `scripts/quality/structure.py` now treats a missing baseline file as
zero exceptions (the #791 end state) instead of reporting a load error; the
metadata contract still applies if a baseline is ever reintroduced.

Evidence: `fmt: checked=8 failed=0 baseline-skipped=0`, `lint: failed=0`
(warnings only) on touched files; raw `fmt --check` ok on all 11 formerly
excepted files; `lint --local` warnings-only on the 4 unchanged ones;
selfhost s2 rebuild ok (20.7s); `verify lane` PASS; `quality structure` PASS
with baseline absent; `docs regenerate`/`docs check` PASS;
`pytest scripts/tests/test_quality_structure.py` 7/7.

Residuals: acceptance item `quality full` (repository-wide fmt+lint) not run
here — it is the ~70min full gate; the per-file evidence above covers every
formerly-excepted file. `manifest_doc_site.ark` remains orphan dead code
(kept: it is the in-compiler reference for re-wiring `doc --html`); tracking
its re-wiring or removal is out of #791 scope.

## Progress receipt — 2026-09-25 (quality full ratchet repair)

- First `quality full` run: fmt `checked=1976 failed=0 baseline-skipped=0`,
  lint `checked=1976 failed=0 baseline-skipped=0` — the formatter-baseline
  removal itself is fully green. The run failed only on the unrelated
  code-quality ratchet: `lines_ge_200 3 > 2` (a 214-char call added by
  an earlier #860 commit) and `thin_wrappers 1754 > 1749` (net +5 drift
  across ~860 branch commits).
- Commit `6cfdc2a3b` restores both ceilings in code (the baseline file is
  intentionally not raised — `with_inventory` forbids count increases):
  six zero-caller pure forwarders removed
  (`MirLocal_type_name_from_table`, `MirLocal_is_string_type`,
  `MirLocal_is_string_or_untyped`, `MirBlock_inst_type_id`,
  `MirModule_replace_function_body`, `corehir_body_expr_count`) and the
  long line wrapped. Standalone `check-ark-code-quality.py` now reports
  `lines_ge_200=2`, `thin_wrappers=1748` → PASS.
- A second `quality full` is running for the end-to-end receipt; result
  will be recorded before this issue moves to done.
