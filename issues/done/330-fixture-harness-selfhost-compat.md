# Fixture harness を selfhost binary 対応にする

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 330
**Depends on**: 328
**Track**: selfhost-retirement
**Blocks v1 exit**: no
**Priority**: 23

## Summary

`tests/harness.rs` が selfhost binary を compiler として使えるようにする。現在 harness は `cargo build -p arukellt` + `target/release/arukellt` 前提でコンパイルされている。

## Current state

- `tests/harness.rs`: Rust binary を直接呼び出し
- compiler binary の path が harness 内部に hardcoded
- selfhost binary でどの fixture が pass / fail するか未測定
- regression tracking の仕組みがない

## Acceptance

- [x] `ARUKELLT_BIN=path/to/selfhost cargo test -p arukellt --test harness` で selfhost compiler が使われる
- [x] selfhost で pass / fail する fixture のリストが生成される
- [x] 差分が regression として追跡可能 (前回の pass リストとの diff)
- [x] pass 率が CI artifact として記録される

## References

- `tests/harness.rs` — fixture harness
- `tests/fixtures/manifest.txt` — fixture 一覧
