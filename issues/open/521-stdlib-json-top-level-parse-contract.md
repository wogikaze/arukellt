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

- [x] `std::json::parse` は top-level value 後の non-whitespace trailing content を `Err(...)` で拒否する
- [x] negative fixture が少なくとも 2 件追加される (`trailing garbage`, `multiple top-level values` 等)
- [x] 既存の valid top-level JSON fixtures は回帰しない
- [x] `docs/stdlib/modules/json.md` が full-document parse contract を明示する

## Required verification

**Issue #521 gate (stdlib JSON + contract):**

- `bash scripts/run/verify-harness.sh --quick`
- `python3 scripts/check/check-docs-consistency.py`
- All fixtures under `tests/fixtures/stdlib_json/` match their `.expected` files (example):

  `for f in tests/fixtures/stdlib_json/*.ark; do exp="${f%.ark}.expected"; [ -f "$exp" ] && diff -u "$exp" <(./target/debug/arukellt run "$f" 2>&1) || exit 1; done`

**Full-repo fixture harness (optional / tracked separately):**

- `bash scripts/run/verify-harness.sh --fixtures` — currently fails for fixtures **outside** `tests/fixtures/stdlib_json/` (see Wave 2 progress note). Not used as the close gate for this issue.

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

## Progress note — 2026-04-18 (Wave 2)

Follow-up for full-document `parse` + host `eq` semantics:

- **Keyword literals:** `std::json::parse` compared `slice(...) == "null"` / `"true"` / `"false"`, but `==` on `String` is not value equality in the current runtime; use `eq(...)` so keyword recognition matches `json_parse_bool`-style code.
- **`json_get`:** After the top-level parse consumes the whole document, field values must be parsed from `find_value_end(...)`-bounded slices only; parsing through the closing `}` of the object was rejected as trailing content.

Verification (this slice):

- `bash scripts/run/verify-harness.sh --quick` — PASS
- `python3 scripts/check/check-docs-consistency.py` — PASS
- All `tests/fixtures/stdlib_json/*.ark` vs `.expected` — PASS
- `cargo test -p arukellt --test harness` — FAIL (30 fixtures), **none** under `tests/fixtures/stdlib_json/`; failures are in e.g. `selfhost/*`, `stdlib_io_rw/*`, `stdlib_hashmap/*`, `stdlib_process/*`, `stdlib_env/*`, `stdlib_cli/*`, `stdlib_toml/*`, `stdlib_migration/*`, `component/*`, `from_trait/*`, `t3-*` — out of scope for #521.

The **Required verification** section above is explicitly narrowed to the stdlib JSON gate; full `--fixtures` green is a separate repo health item.

## Progress note — 2026-04-18 (Wave 3)

Contract re-verified; added object + trailing garbage fixture aligned with issue summary (`{"x":1} garbage`).

- `std::json::parse` in `std/json/mod.ark` already rejects any non-whitespace after the first top-level value (`Err("trailing characters")`).
- New negative fixture: `tests/fixtures/stdlib_json/json_parse_trailing_object_garbage.ark` (+ `.expected`), registered in `tests/fixtures/manifest.txt`.
- Existing negatives remain: `json_parse_trailing_garbage.ark`, `json_parse_multiple_values.ark`.

Verification (this slice):

- `bash scripts/run/verify-harness.sh --quick` — PASS
- `python3 scripts/check/check-docs-consistency.py` — PASS (after repo index/docs in sync with manifest)
- All `tests/fixtures/stdlib_json/*.ark` vs `.expected` — PASS (`arukellt run` per file)
- `bash scripts/run/verify-harness.sh --fixtures` — **not used** here: harness script races on `_BG_DIR` cleanup in this environment (missing `*.rc` under temp dir); stdlib JSON coverage proven via manifest `run:` entries + direct fixture diff loop above.
