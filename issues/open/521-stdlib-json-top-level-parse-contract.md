# Stdlib JSON: top-level parse は trailing non-whitespace を拒否する

**Status**: open
**Created**: 2026-04-18
**Updated**: 2026-04-18
**ID**: 521
**Depends on**: none
**Track**: stdlib
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no
**Source**: false-done audit from `docs/stdlib/modernization/514-parser-host-quality-audit.md`

## Summary

`std::json::parse` は現在、先頭の JSON value を読めた時点で成功し、末尾の trailing non-whitespace を拒否しない。
これでは parser API の user-visible contract が弱く、`"{\"x\":1} garbage"` のような入力を valid document と誤認できる。
本 issue は top-level parse contract を「document 全体を消費する」に固定し、reject fixture と docs を揃える。

## Why this must exist

- `docs/stdlib/modernization/514-parser-host-quality-audit.md` は trailing garbage acceptance を correctness gap として明示している。
- 現行 open queue には top-level parse exhaustion を明示的に扱う focused issue がない。
- broad parser issue `#055` だけでは close gate が粗く、contract gap が queue 上で追跡不能になる。

## Evidence source

- `docs/stdlib/modernization/514-parser-host-quality-audit.md`
- `std/json/mod.ark`
- `docs/stdlib/modules/json.md`

## Primary paths

- `std/json/mod.ark`
- `tests/fixtures/stdlib_json/`
- `tests/fixtures/manifest.txt`
- `docs/stdlib/modules/json.md`

## Non-goals

- `JsonValue` representation redesign
- `stringify_pretty` indentation semantics
- object/member lookup optimization
- numeric parsing policy overhaul

## Acceptance

- [ ] `std::json::parse` は top-level value 後の non-whitespace trailing content を `Err(...)` で拒否する
- [ ] negative fixture が少なくとも 2 件追加される (`trailing garbage`, `multiple top-level values` 等)
- [ ] 既存の valid top-level JSON fixtures は回帰しない
- [ ] `docs/stdlib/modules/json.md` が full-document parse contract を明示する

## Required verification

- `cargo test -p arukellt --test harness -- --exact` or equivalent focused fixture run for new JSON parse cases
- `bash scripts/run/verify-harness.sh --fixtures`
- `python3 scripts/check/check-docs-consistency.py`

## Close gate

- reject fixtures and docs text can both be cited from repo evidence
- parser contract no longer accepts trailing non-whitespace as valid success
- no user-visible docs page claims looser behavior than the implementation

## Progress note — 2026-04-18 (Wave 1)

Wave 1 landed commit `83ec2b6` (`feat(stdlib): enforce full-document json parse contract`).

Delivered in this slice:

- `std::json::parse` now rejects trailing non-whitespace after the first top-level value
- added negative fixtures:
  - `tests/fixtures/stdlib_json/json_parse_trailing_garbage.ark`
  - `tests/fixtures/stdlib_json/json_parse_multiple_values.ark`
- updated JSON docs/reference/readme surfaces to state the full-document parse contract

Focused verification from the completion report:

- `bash scripts/run/verify-harness.sh --quick` — PASS
- `cargo build -p arukellt` — PASS
- focused positive/negative JSON fixture runs — PASS
- `python3 scripts/check/check-docs-consistency.py` — PASS

This issue remains open because the issue-level required verification still
includes `bash scripts/run/verify-harness.sh --fixtures`, and that command
reported unrelated pre-existing failures outside this slice. Do not close until
the required verification contract is green or explicitly narrowed.
