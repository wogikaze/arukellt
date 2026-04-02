# コンパイラパスを抽象化して compiler-agnostic にする

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 328
**Depends on**: 326
**Track**: selfhost-retirement
**Blocks v1 exit**: no
**Priority**: 21

## Summary

scripts / CI workflow が特定の compiler binary に依存しないよう抽象化する。現在 verify-harness.sh, CI job, perf baseline 全てが `target/release/arukellt` を直接参照している。selfhost primary 移行の前提として、compiler binary を差し替え可能にする。

## Current state

- `scripts/run/verify-harness.sh`: Rust binary path を直接使用
- `.github/workflows/ci.yml`: `cargo build -p arukellt` + `./target/release/arukellt` を直接参照
- `scripts/run/run-benchmarks.sh`: Rust binary 前提
- perf baseline: Rust binary の compile time / output size を記録

## Acceptance

- [x] `verify-harness.sh` が `$ARUKELLT_BIN` 環境変数を受け付け、デフォルトは Rust binary
- [x] CI で selfhost / Rust binary を切り替え可能な matrix job が定義される
- [x] perf baseline script が compiler binary を引数で受け付ける
- [x] 切り替え時に fixture 結果の差分が明示的に出力される

## References

- `scripts/run/verify-harness.sh` — verification runner
- `.github/workflows/ci.yml` — CI definition
- `scripts/run/run-benchmarks.sh` — benchmark runner
