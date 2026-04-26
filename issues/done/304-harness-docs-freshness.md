# verify-harness.sh に docs freshness check を追加する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-03
**ID**: 304
**Depends on**: 302
**Track**: docs/ops
**Blocks v1 exit**: no
**Priority**: 24

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: check-docs-freshness.py integrated in verify-harness.sh at line 206-207

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/304-harness-docs-freshness.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`verify-harness.sh` は 13 チェックを実行するが、「docs が stale な状態」を検出して fail する仕組みがない。生成 docs の陳腐化を CI で検出したい。

## Current state

- `scripts/run/verify-harness.sh`: docs consistency check (生成 docs 一致) はある
- bootstrap 状態 / capability 状態の stale は検出しない
- pre-push hook で 17 チェック走るが、docs freshness は含まれない

## Acceptance

- [x] `verify-harness.sh` に `--docs-fresh` チェックが追加される
- [x] project-state.toml の `updated` 日付と current-state.md の実態が整合しない場合 fail
- [x] pre-push hook で docs freshness が検証される

## References

- `scripts/run/verify-harness.sh`
- `docs/data/project-state.toml`
