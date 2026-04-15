# Stdlib: allocation / complexity / perf footgun を family 横断で監査する

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-15
**ID**: 520
**Depends on**: none
**Track**: stdlib
**Blocks v1 exit**: no
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib には repeated `concat`, repeated `slice`, repeated linear scan など、
correctness とは別の performance footgun が残っている。学習用コードとしても実用 surface としても、
allocation と complexity の悪い例を減らし、builder / buffering / better algorithm を優先する方針へ寄せる。

## Repo evidence

- `std/json/mod.ark`, `std/csv/mod.ark`, `std/io/mod.ark`, `std/bytes/mod.ark` には repeated string concat が多い
- parser / formatter 系に repeated `slice` と linear scan が多い
- path / text / bytes family は small helper の組み合わせで O(n^2) になりやすい

## Acceptance

- [ ] family ごとの perf footgun inventory が作成される
- [ ] `concat` 連鎖, linear scan, needless allocation, repeated parse の 4 類型で分類される
- [ ] `text::builder`, buffered I/O, pre-sized vec, better search strategy など推奨置換パターンが決まる
- [ ] benchmark へ繋げるべき hotspot が特定される

## Primary paths

- `std/json/mod.ark`
- `std/csv/mod.ark`
- `std/io/mod.ark`
- `std/bytes/mod.ark`
- `std/text/`

## References

- `issues/done/387-stdlib-bytes-buffered-io-helpers.md`
- `issues/done/385-stdlib-text-unicode-conformance.md`
