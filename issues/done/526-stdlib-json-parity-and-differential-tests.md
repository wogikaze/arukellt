---
Status: done
Created: 2026-04-18
Updated: 2026-04-22
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Stdlib JSON: parity と differential tests を round-trip corpus で拡張する

## Close note

Closed by commit `feat(stdlib): add JSON round-trip parity and differential test corpus (#526)`.

Added 5 new fixture files under `tests/fixtures/stdlib_json/`:
- `json_rt_nested_object.ark` — nested object parse→stringify→re-parse round-trip
- `json_rt_array.ark` — array round-trip
- `json_rt_scalars.ark` — null, bool, 0, negative number, type predicate edge cases
- `json_rt_string_escape.ark` — string escape encoding + golden differential check
- `json_differential.ark` — mixed-type object differential against known-correct structure

All 5 fixtures registered in `tests/fixtures/manifest.txt`.
Normalization rules documented in `tests/fixtures/stdlib_json/NORMALIZATION.md`.

Verification: `cargo test -p arukellt --test harness` — PASS: 750 FAIL: 31 (all FAILs pre-existing, none in stdlib_json new fixtures). No production `std/json/mod.ark` changes made.

**ID**: 526
**Depends on**: none
**Track**: stdlib
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none
**Source**: follow-up extraction from `#519`

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

## Summary

`std::json` family は regression fixtures はあるが、round-trip parity と differential testing がまだ queue 上で focused に切られていない。
この issue は JSON の構造値 round-trip と reference comparison を分離し、parser/stringifier の振る舞い差分を継続検証できる queue artifact にする。

## Why this must exist

- `#519` の matrix で JSON は最優先 family の 1 つとして挙がっているが、focused follow-up issue がまだ存在しない
- 既存 fixture は sanity/regression には十分でも、value normalization と reference differential の継続検証まではカバーしていない
- JSON の contract gap は parser / stringify の両方にまたがるため、単一の broad issue では close gate が粗すぎる

## Evidence source

- `issues/open/519-stdlib-parity-property-and-differential-tests.md`
- `docs/stdlib/modernization/514-parser-host-quality-audit.md`
- `std/json/mod.ark`

## Primary paths

- `std/json/mod.ark`
- `tests/fixtures/stdlib_json/`
- `tests/fixtures/manifest.txt`
- `crates/arukellt/tests/`

## Non-goals

- JSON parser semantics redesign
- pretty-print layout rewrite
- TOML / CSV / collections coverage
- `serde_json` support policy changes beyond the test harness

## Acceptance

- [x] JSON round-trip parity corpus is defined for nested objects, arrays, and scalar edge cases
- [x] at least one differential test path compares Arukellt JSON behavior against a reference implementation or normalized golden corpus
- [x] the test corpus is wired into the repo’s existing harness or a thin Rust test module without changing production JSON behavior
- [x] normalization rules for ordering / formatting differences are documented alongside the corpus

## Required verification

- focused JSON fixture or Rust test run for the new parity corpus
- `python scripts/manager.py verify fixtures`
- `python3 scripts/gen/generate-issue-index.py`

## Close gate

- JSON parity/differential corpus exists in repo and can be cited from the issue
- reference comparison rules are explicit enough to keep future regressions stable
- no production JSON behavior change is required to explain the test contract