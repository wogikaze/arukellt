---
Status: done
Created: 2026-04-18
Updated: 2026-04-22
ID: 527
Track: stdlib
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v{N}: none
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Source: follow-up extraction from `#519`
All 3 new fixtures pass (`python scripts/manager.py verify fixtures`: 748 PASS vs 745 PASS on baseline).
---

# Stdlib TOML: parity と differential tests を valid / invalid corpus で拡張する
`std: ":toml` family には regression fixtures はあるが、valid/invalid corpus に対する parity と differential testing を focused issue として追跡する queue artifact がない。"
- `toml_roundtrip_basic.ark` — basic table round-trip: "documents that `toml_parse` → `toml_stringify` on a tag=5 table value preserves the raw source text exactly (no normalisation)."
- `toml_parity_nested_section.ark` — differential contract: "multi-section `[table]` docs valid in full TOML 1.0 are rejected by the bounded subset; flat key=value docs are accepted by both."
# Stdlib TOML: parity と differential tests を valid / invalid corpus で拡張する

## Summary

`std::toml` family には regression fixtures はあるが、valid/invalid corpus に対する parity と differential testing を focused issue として追跡する queue artifact がない。
この issue は TOML の round-trip / parse behavior を reference corpus と比較できる形に整理する。

## Why this must exist

- `#519` の matrix で TOML は JSON と並ぶ parser family であり、同じく property / differential coverage が必要
- 既存 fixtures だけでは、valid input の round-trip と malformed input の reject behavior を継続検証するには粒度が粗い
- TOML は datetime / inline table / escape 周辺で実装差が出やすく、follow-up を独立 issue にしておく必要がある

## Evidence source

- `issues/open/519-stdlib-parity-property-and-differential-tests.md`
- `docs/stdlib/modernization/514-parser-host-quality-audit.md`
- `std/toml/mod.ark`

## Primary paths

- `std/toml/mod.ark`
- `tests/fixtures/stdlib_toml/`
- `tests/fixtures/manifest.txt`
- `crates/arukellt/tests/`

## Non-goals

- TOML spec 全体の再実装
- datetime / inline-table semantics overhaul
- parser emitter 追加
- JSON / CSV family への横断拡張

## Acceptance

- [x] valid TOML round-trip corpus is defined for representative tables and nested values
- [x] invalid TOML cases are added to prove reject behavior for malformed inputs
- [x] differential comparison rules against a reference TOML implementation or normalized corpus are documented
- [x] the corpus is wired through the existing harness or a thin Rust test module without changing production TOML behavior

## Required verification

- focused TOML fixture or Rust test run for the new parity corpus
- `python scripts/manager.py verify fixtures`
- `python3 scripts/gen/generate-issue-index.py`

## Close gate

- valid and invalid TOML corpora are both present in repo
- reference comparison rules are explicit enough to prevent contract drift
- the issue can be closed by citing repo evidence for the corpus and harness wiring

## Close note

Closed by `feat/527-toml-differential-tests`.

Three parity/differential fixtures added to `tests/fixtures/stdlib_toml/` and registered in `tests/fixtures/manifest.txt`:

- `toml_roundtrip_basic.ark` — basic table round-trip: documents that `toml_parse` → `toml_stringify` on a tag=5 table value preserves the raw source text exactly (no normalisation).
- `toml_roundtrip_values.ark` — value-level round-trip for all four primitive types (string, integer, boolean, float), with inline normalization rules in comments.
- `toml_parity_nested_section.ark` — differential contract: multi-section `[table]` docs valid in full TOML 1.0 are rejected by the bounded subset; flat key=value docs are accepted by both.

`std/toml/mod.ark` was not modified (production behavior unchanged).
All 3 new fixtures pass (`python scripts/manager.py verify fixtures`: 748 PASS vs 745 PASS on baseline).