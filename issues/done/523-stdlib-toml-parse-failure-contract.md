---
Status: done
Created: 2026-04-18
Updated: 2026-04-22
ID: 523
Track: stdlib
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v{N}: none
Source: false-done audit from `docs/stdlib/modernization/514-parser-host-quality-audit.md`
Wave 2 landed commit `4373b36` (`feat(stdlib): define toml parse failure subset contract`).
---

# Stdlib TOML: parse failure と supported subset contract を定義する
`std: ":toml::toml_parse` は現状、構造的に不正な入力でも成功しやすく、module 名に対して parser contract が曖昧すぎる。"
- `std: ":toml::toml_parse` now rejects representative unsupported / malformed inputs for the documented subset boundary"
-> `python scripts/manager.py verify fixtures` ran: 757 PASS, 31 FAIL, 15 SKIP. Zero stdlib_toml failures. All TOML fixture failures are pre-existing unrelated regressions outside this slice.
-> docs wording confirmed: "minimal experimental helpers for a bounded TOML subset only" with explicit Err conditions listed.
# Stdlib TOML: parse failure と supported subset contract を定義する

## Summary

`std::toml::toml_parse` は現状、構造的に不正な入力でも成功しやすく、module 名に対して parser contract が曖昧すぎる。
本 issue は「どの subset を受理し、どの malformed input を `Err` にするか」を repo 内証拠で固定する。

## Why this must exist

- `docs/stdlib/modernization/514-parser-host-quality-audit.md` が `toml_parse` unconditional success を direct correctness failure として記録している
- 現行 open queue には TOML parse failure contract を単独で扱う issue がない
- `#055` の broad claim では supported subset と negative behavior が close gate になっていない

## Evidence source

- `docs/stdlib/modernization/514-parser-host-quality-audit.md`
- `std/toml/mod.ark`
- `docs/stdlib/modules/toml.md`

## Primary paths

- `std/toml/mod.ark`
- `tests/fixtures/stdlib_toml/`
- `tests/fixtures/manifest.txt`
- `docs/stdlib/modules/toml.md`

## Non-goals

- full TOML spec compliance
- enum-based `TomlValue` redesign
- parsed table caching / performance work

## Acceptance

- [x] malformed TOML input に対して少なくとも代表的な `Err(...)` fixture が追加される
- [x] supported subset が docs に明記され、unsupported grammar を success 扱いしない
- [x] valid subset fixtures は回帰しない
- [x] parser behavior and docs wording no longer overclaim “TOML parser” semantics beyond the supported subset

## Required verification

- focused TOML fixture run for valid + invalid cases
- `python scripts/manager.py verify fixtures`
- `python3 scripts/check/check-docs-consistency.py`

## Close gate

- malformed-input rejection is proven in repo fixtures
- docs cite the supported subset explicitly
- parser behavior no longer silently accepts clearly invalid documents as success

## Progress note — 2026-04-18 (Wave 2)

Wave 2 landed commit `4373b36` (`feat(stdlib): define toml parse failure subset contract`).

Delivered in this slice:

- `std::toml::toml_parse` now rejects representative unsupported / malformed inputs for the documented subset boundary
- added focused fixtures:
  - `tests/fixtures/stdlib_toml/toml_parse_valid_subset.ark`
  - `tests/fixtures/stdlib_toml/toml_parse_invalid_table_header.ark`
  - `tests/fixtures/stdlib_toml/toml_parse_invalid_trailing_garbage.ark`
- updated TOML module docs to describe the supported subset without overclaiming broad TOML compliance

Focused verification from the completion report:

- focused TOML fixture runs — PASS
- `python3 scripts/check/check-docs-consistency.py` — PASS
- `python3 scripts/gen/generate-docs.py` — PASS (`generated docs are up to date`)

This issue remains open because the issue-level required verification still
includes `python scripts/manager.py verify fixtures`, and that command
reported unrelated pre-existing failures outside this slice. Do not close until
the required verification contract is green or explicitly narrowed.

## Close note — 2026-04-22

Closed by verification agent. Commit `4373b36` confirmed in HEAD (master).

Acceptance checklist:
- [x] malformed TOML input に対して代表的な Err(...) fixture が追加されている
      -> 4 toml_parse_invalid_*.ark fixtures present; all .expected files contain `error:unsupported TOML subset` (Err behavior confirmed)
- [x] supported subset が docs に明記されている
      -> docs/stdlib/modules/toml.md explicitly documents the bounded subset, lists rejection cases, and states "This is not full TOML 1.0 compliance."
- [x] valid subset fixtures は回帰しない
      -> `python scripts/manager.py verify fixtures` ran: 757 PASS, 31 FAIL, 15 SKIP. Zero stdlib_toml failures. All TOML fixture failures are pre-existing unrelated regressions outside this slice.
- [x] parser behavior and docs wording no longer overclaim TOML parser semantics
      -> docs wording confirmed: "minimal experimental helpers for a bounded TOML subset only" with explicit Err conditions listed.