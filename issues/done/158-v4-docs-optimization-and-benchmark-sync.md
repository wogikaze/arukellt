# v4 docs 完了: optimization / pipeline / current-state / benchmark caveat の同期

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 158
**Depends on**: 140, 141, 142, 143, 145, 148, 155
**Track**: cross-cutting
**Blocks v1 exit**: no

## Summary

`docs/process/roadmap-v4.md` §11 は v4 で必ず残すドキュメントとして
`docs/compiler/pipeline.md`, `docs/current-state.md`, `docs/process/benchmark-results.md`, `docs/migration/v3-to-v4.md`
を挙げている。
`docs/migration/v3-to-v4.md` 自体は #155 で追跡されているが、v4 最適化の最終状態と
benchmark 結果の caveat（特に §10 item 7 の `binary_tree` 1.83x 問題の記録）を横断して同期する issue は未作成である。

## 受け入れ条件

1. `docs/compiler/pipeline.md` に MIR / backend 最適化パスの現状と opt-level 境界が反映される
2. `docs/current-state.md` に v4 optimization / benchmark 状態の要約が反映される
3. `docs/process/benchmark-results.md` に benchmark summary と必要な caveat / 回避策が反映される
4. `docs/migration/v3-to-v4.md` と上記文書の参照関係が整合し、利用者が移行観点から辿れる

## 実装タスク

1. v4 最適化関連の done/open issue を棚卸しし、docs に残すべき current contract を整理する
2. `docs/compiler/pipeline.md` に MIR pass / backend peephole / dump / opt-level の現状を反映する
3. `docs/process/benchmark-results.md` に compile/runtime/size/memory の結果と caveat を反映する
4. `binary_tree` が依然 worst-case なら、理由と回避策を `docs/process/benchmark-results.md` に明記する
5. `docs/current-state.md` と `docs/migration/v3-to-v4.md` の導線を整える

## 参照

- `docs/process/roadmap-v4.md` §10 item 7
- `docs/process/roadmap-v4.md` §11
- `issues/open/140-bench-one-command-workflow.md`
- `issues/open/141-bench-compile-latency-breakdown.md`
- `issues/open/142-bench-runtime-latency-throughput.md`
- `issues/open/143-bench-memory-gc-telemetry.md`
- `issues/open/145-bench-size-attribution-and-diff.md`
- `issues/open/148-bench-result-storage-and-trend-report.md`
- `issues/open/155-selfhosting-checklist-and-migration-docs.md`
