# Perf baseline を selfhost binary 対応にする

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-01
**Closed**: 2026-04-01
**ID**: 331
**Depends on**: 330
**Track**: selfhost-retirement
**Blocks v1 exit**: no
**Priority**: 24

## Summary

benchmarks/ の baseline と比較を selfhost binary でも実行可能にする。Rust binary との性能差を可視化し、selfhost primary 移行の判断材料にする。

## Resolution

Implemented `--compare` and `--selfhost` flags in `scripts/run/run-benchmarks.sh`:
- `--compare`: runs both Rust and selfhost compilers, shows side-by-side comparison table
- `--selfhost`: runs benchmarks using selfhost compiler only
- Compile time ratio and binary size ratio displayed in summary
- Warning emitted when selfhost is >200% slower
- Added `mise bench:compare` and `mise bench:selfhost` tasks
- Fixed median() to return integer (bash arithmetic compatibility)

Verification: `bash scripts/run/run-benchmarks.sh --quick --compare` — runs successfully, shows fib/binary_tree/string_concat at 1.1-1.7x compile overhead with 0.48-0.54x binary size.

## Acceptance

- [x] `mise bench` が compiler binary を引数で受け付ける
- [x] selfhost binary の compile time / output size がベースラインに記録される
- [x] Rust binary との性能差が summary に表示される
- [x] 性能劣化が一定閾値を超えた場合に warning を出す
