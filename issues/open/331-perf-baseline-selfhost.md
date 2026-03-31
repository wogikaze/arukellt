# Perf baseline を selfhost binary 対応にする

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 331
**Depends on**: 330
**Track**: selfhost-retirement
**Blocks v1 exit**: no
**Priority**: 24

## Summary

benchmarks/ の baseline と比較を selfhost binary でも実行可能にする。Rust binary との性能差を可視化し、selfhost primary 移行の判断材料にする。

## Current state

- `scripts/run-benchmarks.sh`: Rust binary 前提のコマンド列
- `tests/baselines/perf/current.json` / `baselines.json`: Rust binary の結果
- selfhost binary の compile time / output size / runtime が未測定
- 性能差の可視化がない

## Acceptance

- [ ] `mise bench` が compiler binary を引数で受け付ける
- [ ] selfhost binary の compile time / output size がベースラインに記録される
- [ ] Rust binary との性能差が summary に表示される
- [ ] 性能劣化が一定閾値を超えた場合に warning を出す

## References

- `scripts/run-benchmarks.sh` — benchmark runner
- `tests/baselines/perf/` — perf baselines
- `benchmarks/` — benchmark suite
