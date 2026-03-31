# Workspace 構造を selfhost primary に備えて再編する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 332
**Depends on**: 329
**Track**: selfhost-retirement
**Blocks v1 exit**: no
**Priority**: 25

## Summary

Cargo.toml の default-members と workspace 構造を、selfhost primary への移行に備えて再編する計画を策定する。現在は compiler pipeline crate が default-members の中心にあるが、selfhost primary 後はこれらが optional になる。

## Current state

- `Cargo.toml` default-members: ark-driver, ark-mir, ark-wasm, arukellt を含む
- 全ての CI / build / test が Cargo workspace 前提
- selfhost primary 後に default-members から compiler pipeline を外す必要がある
- IDE tooling (ark-lsp, ark-dap) は Rust のまま残る可能性

## Acceptance

- [ ] compiler-pipeline crate を optional feature gate 配下に移す計画が文書化される
- [ ] selfhost primary 後に残る minimal Cargo.toml 構造が設計される
- [ ] 移行手順が step-by-step で定義される
- [ ] IDE tooling crate の位置付けが明確化される

## References

- `Cargo.toml` — workspace 定義
- `docs/compiler/bootstrap.md` — retained crates
- `crates/` — 全 Rust crate
