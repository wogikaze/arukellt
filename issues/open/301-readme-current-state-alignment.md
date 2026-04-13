# current-state.md と README.md の整合を取る

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-04-13
**ID**: 301
**Depends on**: 303
**Track**: docs/ops
**Blocks v1 exit**: no
**Priority**: 21

---


## Reopened by audit — 2026-04-13

**Reason**: README/current-state fixture counts differ.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: generate-docs.py generates README/current-state, check-docs-consistency.py passes

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/301-readme-current-state-alignment.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

README.md と current-state.md の間に fixture count 等の数値ズレがある。project-state.toml を source of truth として両方を再生成し、一致させる。

## Current state

- README.md: 588 entries と記載
- current-state.md: 586 entries と記載
- `docs/data/project-state.toml:81`: `fixture_manifest_count = 586`
- `python3 scripts/gen/generate-docs.py` で再生成すれば解消するはず

## Acceptance

- [x] `python3 scripts/gen/generate-docs.py` で再生成し、README / current-state の数値が一致
- [x] project-state.toml の値が実態 (manifest.txt の行数) と一致
- [x] `scripts/check/check-docs-consistency.py` が pass

## References

- `README.md`
- `docs/current-state.md`
- `docs/data/project-state.toml`
- `scripts/gen/generate-docs.py`
