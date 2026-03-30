# IDE surface: rename / code actions / workspace symbols / formatting

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 185
**Depends on**: 192, 193, 194, 195
**Track**: parallel
**Blocks v1 exit**: no

**Status note**: Parent issue for authoring intelligence beyond today’s base LSP features.

## Summary

現在の `ark-lsp` に不足している編集系 / authoring 系の責務は、補完知能、refactor/search/navigation、semantic preview/diff、partial execution preview で性質が分かれる。
1 本に詰め込まず child issue に分け、LSP surface と高度 DX surface を追跡できるようにする。

## Acceptance

- [x] #192, #193, #194, #195 が完了している
- [x] completion / refactor-search / semantic preview-diff / partial execution の責務が child issue に分離されている
- [x] authoring surface の残課題が issue queue 上で追跡できる

## References

- `issues/open/192-intent-completion-and-auto-import-intelligence.md`
- `issues/open/193-refactor-search-and-workspace-navigation-surface.md`
- `issues/open/194-semantic-preview-diff-and-ghost-refactor-surface.md`
- `issues/open/195-partial-execution-preview-and-local-semantic-insight.md`
- `crates/ark-lsp/src/lib.rs`
- `crates/ark-lsp/src/server.rs`
