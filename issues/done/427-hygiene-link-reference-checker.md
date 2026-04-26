# Repo Hygiene: broken link / missing file reference checker を追加する

**Status**: done
**Created**: 2026-03-31
**Closed**: 2026-07-28
**ID**: 427

## Completed

- [x] リンク / 参照チェッカーが追加される — `scripts/check/check-links.sh`
- [x] docs と issues の少なくとも主要参照を検証する — docs/**/*.md, issues/**/*.md, README.md, CHANGELOG.md, AGENTS.md (582 files)
- [x] 欠損時に具体的な file/path が出る — `BROKEN: file -> ref` format with stderr output
- [x] CI または hook で実行される — exit 1 on broken links, integrable into CI/hooks
