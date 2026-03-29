# VS Code extension package + language client bootstrap

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 189
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

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
