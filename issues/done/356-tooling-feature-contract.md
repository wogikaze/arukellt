# Tooling Contract: CLI / LSP / Extension の機能契約を文書化する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 356
**Depends on**: 355
**Track**: tooling-contract
**Blocks v1 exit**: no
**Priority**: 24

## Summary

`fmt` / `check` / `lint` / go-to-definition / references / rename / hover / auto-import / organize imports の各機能について、CLI / LSP / VS Code extension のどこが source of truth かを定義し、機能差が意図的かバグかを判定可能にする。

## Current state

- CLI と LSP で `fmt` の出力が同じことの保証なし
- LSP の機能と extension の command/capability の対応が文書化されていない
- `docs/test-strategy.md` は editor-tooling を smoke planned 扱い
- どの機能が editor-only でどの機能が CLI にもあるかの明示なし

## Acceptance

- [x] tooling feature matrix (機能 × CLI / LSP / Extension の対応表) が `docs/` に存在する
- [x] 各機能の source of truth (shared code path / LSP-only / CLI-only) が明記される
- [x] matrix 内の各機能に対応するテスト ID が参照される
- [x] CLI `fmt` と LSP `formatting` の出力一致がテストで保証される

## References

- `docs/test-strategy.md` — テスト戦略
- `crates/arukellt/src/commands.rs` — CLI 機能
- `crates/ark-lsp/src/server.rs` — LSP 機能
- `extensions/arukellt-all-in-one/package.json` — extension capabilities
