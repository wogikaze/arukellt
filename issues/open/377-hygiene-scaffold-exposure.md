# Repo Hygiene: scaffold / internal コンポーネントの露出 tier を定める

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 377
**Depends on**: —
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 20

## Summary

`ark-lsp`、`ark-llvm`、selfhost scaffold、not-started target など、product surface としては未完成だが repo に存在するコンポーネントの露出ポリシーを定める。README / docs / Cargo.toml の default-members で、利用者向け surface と internal 開発資産を視覚的に区別する。

## Current state

- `README.md` の workspace overview に `ark-lsp`、`ark-llvm` が表に出ている
- `Cargo.toml` の default-members / workspace members に全 crate が列挙
- `crates/ark-llvm/` は LLVM 18 必須で通常ビルドから除外されるが workspace に存在
- target registry に `wasm32-freestanding` が `implemented: false` で登録
- 利用者向け surface と internal の区別が明示されていない

## Acceptance

- [ ] コンポーネントの露出 tier (product / internal / scaffold / experimental) が定義される
- [ ] `README.md` が product tier のみを主要セクションで紹介する
- [ ] internal / scaffold tier のコンポーネントに README 内注記が付与される
- [ ] `docs/current-state.md` が tier に準じた表記を使用する

## References

- `README.md` — workspace overview
- `Cargo.toml` — workspace members
- `crates/ark-llvm/` — LLVM backend (internal)
- `crates/ark-lsp/` — LSP (scaffold)
- `docs/current-state.md` — project state
