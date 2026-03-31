# Tooling Contract: ark-lsp を CI merge gate に含める

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 353
**Depends on**: —
**Track**: tooling-contract
**Blocks v1 exit**: no
**Priority**: 21

## Summary

`cargo clippy` と `cargo test` から `--exclude ark-lsp` を外し、ark-lsp を merge-blocking な CI gate に含める。現在 LSP は CI から完全に除外されているため、LSP を壊す変更がそのまま merge される。

## Current state

- `.github/workflows/ci.yml:57`: `cargo clippy --workspace --exclude ark-llvm --exclude ark-lsp`
- `.github/workflows/ci.yml:64`: `cargo test --workspace --exclude ark-llvm --exclude ark-lsp`
- `docs/release-checklist.md` も `--exclude ark-lsp` 前提
- ark-lsp の既存テスト (3 件) が CI で実行されていない

## Acceptance

- [ ] CI の clippy step から `--exclude ark-lsp` が除去される
- [ ] CI の test step から `--exclude ark-lsp` が除去される
- [ ] ark-lsp が clippy warning なしで pass する
- [ ] ark-lsp の既存テストが CI で pass する
- [ ] `docs/release-checklist.md` の除外リストから `ark-lsp` が除去される

## References

- `.github/workflows/ci.yml:57,64` — `--exclude ark-lsp`
- `docs/release-checklist.md` — release gate
- `crates/ark-lsp/src/server.rs` — LSP 実装
