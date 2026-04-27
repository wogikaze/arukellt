---
Status: done
Created: 2026-03-29
Updated: 2026-04-05
ID: 155
Track: cross-cutting
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: False
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
# 横断 docs: selfhosting stdlib checklist と不足 migration guides を整備
---
# 横断 docs: selfhosting stdlib checklist と不足 migration guides を整備

---

## Reopened by audit — 2026-04-03

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/155-selfhosting-checklist-and-migration-docs.md` — incorrect directory for an open issue.

## Summary

`docs/process/roadmap-cross-cutting.md` §6.4 は
- `docs/process/selfhosting-stdlib-checklist.md`
- `docs/migration/v2-to-v3.md`
- `docs/migration/v3-to-v4.md`
- `docs/migration/v4-to-v5.md`
を最終的に要求している。
現状は `v1-to-v2.md` と `t1-to-t3.md` はあるが、後続 migration と selfhosting checklist が未整備。

注: CHANGELOG.md は v0.1 段階のため削除済み（2026-04-28）。

## 受け入れ条件

1. `docs/process/selfhosting-stdlib-checklist.md` が追加され、v5 に必要な stable stdlib surface を追跡できる
2. `docs/migration/v2-to-v3.md`, `docs/migration/v3-to-v4.md`, `docs/migration/v4-to-v5.md` の雛形または初版が揃う
3. process / stdlib docs から各 migration/checklist に辿れる

## 実装タスク

1. selfhosting に必要な stdlib / compiler capability を checklist 化する
2. migration guides の共通フォーマットを定義する
3. 既存 docs から拾える変更履歴を初期投入する
4. verify-harness で最低限ファイル存在をチェックするか、docs consistency に組み込む

## 参照

- `docs/process/roadmap-cross-cutting.md` §6.4, §6.6
- `docs/migration/v1-to-v2.md`
- `docs/stdlib/reference.md`
- `docs/process/roadmap-v3.md`
- `docs/process/roadmap-v4.md`
- `docs/process/roadmap-v5.md`

---

## Closed by orchestrator — 2026-04-05

Close gate satisfied (commit `3c2757f`):
- docs/migration/v2-to-v3.md created
- docs/migration/v3-to-v4.md created
- docs/migration/v4-to-v5.md and docs/process/selfhosting-stdlib-checklist.md already existed
- verify-harness.sh --quick 19/19

Updated 2026-04-28: CHANGELOG.md removed (project at v0.1, changelog not needed yet).
