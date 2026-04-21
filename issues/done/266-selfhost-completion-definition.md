# selfhost 完了条件を定義し文書に固定する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-04-14
**ID**: 266
**Depends on**: 253
**Track**: main
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #253
**Blocks v1 exit**: yes

---

## Reopened by audit — 2026-04-13

**Reason**: Completion not achieved.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: docs/compiler/bootstrap.md has Stage 0/1/2 checklist for selfhost completion

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/266-selfhost-completion-definition.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

「selfhost できたかどうか」に対して、資料ごとに異なる答えが返る状態が続いている。完了条件を 1 行で言える形で文書に固定し、今後の判断基準とする。

## Acceptance

- [x] `docs/compiler/bootstrap.md` に selfhost 完了条件が「checklist」として明記されている
- [x] 完了条件が `Stage0→Stage1→Stage2 fixpoint` / `Stage1 fixture parity` / `CLI parity` / `diagnostic parity` / `determinism` の 5 項目で構成されている
- [x] 各項目の「達成 / 未達」を判定するコマンドまたは CI ジョブが対応付けられている
- [x] `docs/current-state.md` の selfhost セクションがこの checklist を参照している

## Scope

- `docs/compiler/bootstrap.md` の完了条件セクションを新規作成または書き直し
- 各条件に対応する検証コマンドを記載
- `docs/current-state.md` から `docs/compiler/bootstrap.md` への参照に変換

## References

- `docs/compiler/bootstrap.md`
- `docs/migration/v4-to-v5.md`
- `scripts/run/verify-bootstrap.sh`
- `issues/open/253-selfhost-completion-criteria.md`

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
