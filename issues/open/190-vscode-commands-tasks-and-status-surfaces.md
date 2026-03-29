# VS Code commands / tasks / status surfaces

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 190
**Depends on**: 189
**Track**: parallel
**Blocks v1 exit**: no

## Summary

extension package の上で、Command Palette、task provider、status bar、output channel、restart-LSP、target / emit 設定 handoff を整備する。language client bootstrap とは別の command surface として追う。

## Acceptance

- [ ] `check` / `compile` / `run` / restart-LSP の command surface が追跡できる
- [ ] task provider / status bar / output channel の責務が整理されている
- [ ] target / emit / adapter などの設定 handoff を issue queue 上で追跡できる

## References

- `issues/open/184-vscode-extension-foundation.md`
- `issues/open/189-vscode-extension-package-and-language-client-bootstrap.md`
- `crates/arukellt/src/commands.rs`
