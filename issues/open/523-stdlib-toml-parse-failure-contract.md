# Stdlib TOML: parse failure と supported subset contract を定義する

**Status**: open
**Created**: 2026-04-18
**Updated**: 2026-04-18
**ID**: 523
**Depends on**: none
**Track**: stdlib
**Blocks v1 exit**: no
**Source**: false-done audit from `docs/stdlib/modernization/514-parser-host-quality-audit.md`

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

- [ ] malformed TOML input に対して少なくとも代表的な `Err(...)` fixture が追加される
- [ ] supported subset が docs に明記され、unsupported grammar を success 扱いしない
- [ ] valid subset fixtures は回帰しない
- [ ] parser behavior and docs wording no longer overclaim “TOML parser” semantics beyond the supported subset

## Required verification

- focused TOML fixture run for valid + invalid cases
- `bash scripts/run/verify-harness.sh --fixtures`
- `python3 scripts/check/check-docs-consistency.py`

## Close gate

- malformed-input rejection is proven in repo fixtures
- docs cite the supported subset explicitly
- parser behavior no longer silently accepts clearly invalid documents as success
