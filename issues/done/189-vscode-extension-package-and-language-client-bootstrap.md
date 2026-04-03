# VS Code extension package + language client bootstrap

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-03
**ID**: 189
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no


---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: extension.js has arukellt lsp launch, language registration in package.json, binary discovery all tracked

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).


## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/189-vscode-extension-package-and-language-client-bootstrap.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`arukellt-all-in-one` extension package を作成し、`.ark` language registration、basic grammar / snippets、`arukellt lsp` への接続、binary discovery の最小土台を整える。foundation 系 child issue の起点。

## Acceptance

- [x] extension package と `.ark` language registration が追跡できる
- [x] `arukellt lsp` を起動する language client 導線がある
- [x] binary discovery / settings override / version check の責務が定義されている

## References

- `issues/open/184-vscode-extension-foundation.md`
- `crates/ark-lsp/src/lib.rs`
- `crates/arukellt/src/main.rs`
