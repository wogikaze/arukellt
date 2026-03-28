# ベンチマーク比較: C/Rust/Go/Grain との自動比較スクリプト

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 112
**Depends on**: 109
**Track**: benchmark
**Blocks v4 exit**: yes

## Summary

roadmap-v4.md §5.4 で選定した比較言語 (C gcc-O2 / Rust --release / Go / Grain) との
ベンチマーク比較を自動化するスクリプトを `scripts/compare-benchmarks.sh` として追加する。

## 受け入れ条件

1. `scripts/compare-benchmarks.sh` が各言語のバイナリをビルドして hyperfine で計測
2. 結果を `docs/process/benchmark-results.md` に Markdown テーブルとして出力
3. C 比 (fib: 1.5x 以内, vec-ops: 2.0x 以内) の合否を自動判定
4. Grain (Wasm-native GC 言語) との比較で Arukellt の優位性を確認

## 参照

- roadmap-v4.md §5.4
