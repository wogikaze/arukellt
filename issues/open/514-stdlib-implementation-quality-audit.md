# Stdlib: 実装品質監査 (hash / parsing / collection algorithm の甘さ) を実施する

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-15
**ID**: 514
**Depends on**: none
**Track**: stdlib
**Blocks v1 exit**: no
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib には「とりあえず動く」実装が残っており、hash quality や parser robustness、
data structure invariants の品質面で Rust 標準実装や一般的期待値との差が大きい箇所がある。
この issue は correctness / collision / robustness / invariants の観点から実装品質を監査し、
優先順位つきの follow-up を切り出す。

## Repo evidence

- `std/core/hash.ark` は simple multiplicative hash を使っている
- `std/collections/hash.ark` は simple hash + linear probing 前提で、quality note が弱い
- `std/fs/mod.ark` には best-effort / stub notice が残る

## Acceptance

- [ ] hash family, parser family, collection family, host facade family の品質監査リストが作成される
- [ ] correctness risk / perf risk / collision risk / contract ambiguity の 4 軸で優先順位が付く
- [ ] 少なくとも `std::core::hash`, `std::collections::hash`, `std::json`, `std::toml`, `std::fs` の監査結果が文書化される
- [ ] 高優先度の follow-up issue が必要数だけ派生するか、本 issue 内に subtask として整理される

## Primary paths

- `std/core/hash.ark`
- `std/collections/hash.ark`
- `std/json/mod.ark`
- `std/toml/mod.ark`
- `std/fs/mod.ark`

## References

- `issues/done/044-std-collections-hash.md`
- `issues/done/392-stdlib-error-result-conventions.md`
