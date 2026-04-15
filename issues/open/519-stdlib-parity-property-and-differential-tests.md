# Stdlib: property / differential / parity test を family 横断で拡張する

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-15
**ID**: 519
**Depends on**: none
**Track**: stdlib
**Blocks v1 exit**: no
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib の現在の fixture 群は regression には有効だが、境界条件・代数法則・実装差分検出には弱い。
JSON/TOML/CSV/collections/text/path/hash などに property test / round-trip / differential test の観点を導入し、
「なんとなく動く」実装を継続的に押し上げる。

## Repo evidence

- round-trip baseline は family 限定で、hash / bytes / path / io の法則検証は薄い
- `std/core/hash.ark` や `std/collections/hash.ark` は collision / parity の継続検証が弱い
- parser families は malformed input / normalization / lossy cases の coverage が uneven

## Acceptance

- [ ] family ごとの test strategy matrix (regression / property / differential / fuzz-ish) が作成される
- [ ] `json`, `toml`, `csv`, `path`, `collections/hash`, `text` の優先ケースが列挙される
- [ ] どの family は Rust or external reference と比較できるかが整理される
- [ ] follow-up fixture / harness issue が必要なら派生する

## Primary paths

- `tests/fixtures/stdlib_*`
- `tests/harness.rs`
- `std/json/mod.ark`
- `std/toml/mod.ark`
- `std/collections/hash.ark`

## References

- `issues/done/389-stdlib-serialization-roundtrip-baselines.md`
- `issues/done/388-stdlib-collections-seq-parity-tests.md`
