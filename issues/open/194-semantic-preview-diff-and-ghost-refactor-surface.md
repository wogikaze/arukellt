# Semantic preview / diff / ghost refactor surface

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-04-13
**ID**: 194
**Depends on**: 193
**Track**: parallel
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: No semantic preview/diff/ghost refactor implementation found in LSP or extension.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

text diff を超えて AST / symbol / behavior diff を提示し、rename / format / organize imports / structural rewrite を仮想適用で preview できるようにする。refactor の見える化専用の child issue。

## Acceptance

- [x] semantic diff と behavior-aware diff の責務が追跡できる
- [x] ghost refactor / preview-only apply の導線が定義されている
- [x] refactor 前後の見える化 UX を issue queue 上で追跡できる

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `issues/open/193-refactor-search-and-workspace-navigation-surface.md`
- `crates/ark-lsp/src/server.rs`
