# 横断検証: snapshot / baseline 更新導線と verify-harness 統合

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 152
**Depends on**: —
**Track**: cross-cutting
**Blocks v1 exit**: no

## Summary

`docs/process/roadmap-cross-cutting.md` §6.5 は
- `tests/snapshots/mir/`
- `tests/snapshots/diagnostics/`
- `scripts/update-snapshots.sh`
- baseline の更新運用
を要求している。
現状は `tests/baselines/perf/` と `scripts/collect-baseline.py` はあるが、snapshot 更新導線と verify-harness の役割分担が未整理。

## 受け入れ条件

1. MIR / diagnostics snapshot の置き場と更新規約が定義される
2. `scripts/update-snapshots.sh` が追加され、対象 snapshot を一括更新できる
3. baseline と snapshot の責務分担が `docs/process/benchmark-plan.md` または process docs に明記される
4. `scripts/verify-harness.sh` が snapshot/baseline 前提の check を壊れない形で実行できる

## 実装タスク

1. 現在の `ARUKELLT_DUMP_PHASES` / diagnostics dump と baseline 保存先の対応を棚卸しする
2. MIR / diagnostics snapshot の最小セットを決める
3. `scripts/update-snapshots.sh` を追加し、運用ドキュメントを更新する
4. verify-harness と docs consistency check の役割分担を整理する

## 参照

- `docs/process/roadmap-cross-cutting.md` §6.5
- `docs/compiler/diagnostics.md`
- `docs/compiler/pipeline.md`
- `scripts/verify-harness.sh`
- `scripts/collect-baseline.py`
