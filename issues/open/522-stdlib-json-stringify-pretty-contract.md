# Stdlib JSON: `stringify_pretty` の product claim を現実に合わせる

**Status**: open
**Created**: 2026-04-18
**Updated**: 2026-04-18
**ID**: 522
**Depends on**: none
**Track**: stdlib
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no
**Source**: false-done audit from `docs/stdlib/modernization/514-parser-host-quality-audit.md`

## Summary

`std::json::stringify_pretty(v, indent)` は名前上 pretty-print semantics を約束しているが、
current docs は「basic pass-through; full indentation deferred」と記している。
この mismatch は user-visible claim の先行であり、implementation か naming/docs contract のどちらかを揃える必要がある。

## Why this must exist

- `docs/stdlib/modules/json.md` が未実装の pretty semantics を明示している
- `docs/stdlib/modernization/514-parser-host-quality-audit.md` が focused follow-up を要求している
- 現行 open queue に `stringify_pretty` だけを close gate 付きで扱う issue がない

## Evidence source

- `docs/stdlib/modules/json.md`
- `docs/stdlib/modernization/514-parser-host-quality-audit.md`
- `std/json/mod.ark`

## Primary paths

- `std/json/mod.ark`
- `docs/stdlib/modules/json.md`
- `docs/stdlib/reference.md`
- `tests/fixtures/stdlib_json/`
- `tests/fixtures/manifest.txt`

## Non-goals

- full JSON parser redesign
- object/array internal representation redesign
- cross-format pretty printers for TOML / CSV

## Acceptance

- [ ] repo chooses one contract explicitly:
- [ ] either `stringify_pretty` implements indentation/newline semantics with fixture proof
- [ ] or docs/reference/name text are downgraded so no pretty-print claim remains ahead of reality
- [ ] chosen contract is reflected consistently in module docs, reference docs, and fixture evidence

## Required verification

- focused JSON fixture run proving the chosen outcome
- `python3 scripts/check/check-docs-consistency.py`
- if implementation changes: `bash scripts/run/verify-harness.sh --fixtures`

## Close gate

- no docs / reference page claims indentation semantics without repo proof
- callable surface and docs text say the same thing
- evidence file(s) for either implementation or relabel outcome are checked in
