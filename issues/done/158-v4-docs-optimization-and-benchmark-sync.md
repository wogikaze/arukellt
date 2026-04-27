---
Status: done
Created: 2026-03-29
Updated: 2026-04-03
ID: 158
Track: cross-cutting
Depends on: 140, 141, 142, 143, 145, 148, 155
Orchestration class: implementation-ready
---
# v4 docs 完了: optimization / pipeline / current-state / benchmark caveat の同期
**Blocks v1 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/158-v4-docs-optimization-and-benchmark-sync.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

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