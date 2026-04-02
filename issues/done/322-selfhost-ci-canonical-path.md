# Selfhost CLI を CI canonical path として使えるようにする

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-08
**ID**: 322
**Depends on**: 320, 321
**Track**: selfhost-cli
**Blocks v1 exit**: no
**Priority**: 19

## Summary

verify-harness.sh や CI workflow が selfhost binary を compiler として使えるようにする。現在は全て `target/release/arukellt` (Rust binary) 前提。

## Current state

- `scripts/verify-harness.sh`: Rust binary を直接参照
- `.github/workflows/ci.yml`: `cargo build -p arukellt` → `./target/release/arukellt`
- `tests/harness.rs`: Rust binary 前提のテスト構造
- selfhost binary を代替指定する仕組みがない

## Acceptance

- [x] `verify-harness.sh` が `ARUKELLT_BIN` 環境変数で selfhost binary を受け付ける
- [x] CI に `selfhost-harness` job が追加される (informational, non-blocking)
- [x] 少なくとも基本 fixture 50 個が selfhost compiler で pass する
- [x] pass / fail リストが CI artifact として保存される

## References

- `scripts/verify-harness.sh` — verification runner
- `.github/workflows/ci.yml` — CI definition
- `tests/harness.rs` — fixture harness
