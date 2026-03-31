# VSCode Extension: Workspace / Package / ark.toml を理解した project-aware editor にする

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 441
**Depends on**: 333, 335, 340
**Track**: vscode-ide
**Blocks v1 exit**: no
**Priority**: 3

## Summary

VSCode拡張を単なるファイル単位ツールから、ark.toml・workspace・package構成を理解した project-aware editor にする。import解決、package境界、multi-root workspace に対応する。

## Current state

- `project_root` は取得するが活用していない。
- `ark.toml` は再読込のみ。
- package/workspace 概念が LSP に反映されていない。

## Acceptance

- [ ] `ark.toml` を元に module graph を構築する。
- [ ] workspace 内複数 package を解決可能にする。
- [ ] cross-package go-to-definition が動作する。
- [ ] import 解決が package aware になる。
- [ ] workspace 変更時に index が再構築される。

## References

- `crates/ark-manifest/src/lib.rs`
- `docs/ark-toml.md`
- `crates/ark-lsp/src/server.rs`
- `tests/package-workspace/`
