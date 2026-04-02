# CI perf gate: コンパイル時間・実行時間・バイナリサイズ閾値チェック

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 110
**Depends on**: 109
**Track**: benchmark
**Blocks v4 exit**: yes

## Summary

`scripts/run/verify-harness.sh` を拡張して、
コンパイル時間・実行時間・バイナリサイズの回帰を CI で自動検知する perf gate を追加する。
roadmap-v4.md §6 item 8 および §7 で明示的に要求されている。

## 閾値定義

| 軸 | 閾値 (ベースライン比) | アクション |
|----|---------------------|-----------|
| コンパイル時間 | +20% | failure |
| 実行時間 | +10% | failure |
| バイナリサイズ | +15% | failure |

## 受け入れ条件

1. `tests/baselines/perf/` に JSON 形式のベースラインファイル
2. `scripts/run/verify-harness.sh` に `--perf-gate` オプション追加
3. `scripts/update-baselines.sh` でベースラインを手動更新
4. CI で perf gate が失敗した場合にわかりやすいエラーメッセージ

## 参照

- roadmap-v4.md §6 item 8, §7, §8
