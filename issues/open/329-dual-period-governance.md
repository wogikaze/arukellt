# Dual-period governance を確定する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 329
**Depends on**: 328
**Track**: selfhost-retirement
**Blocks v1 exit**: no
**Priority**: 22

## Summary

bootstrap.md に記載されている「残す Rust crate / 消す Rust crate」の境界を固定し、実行順を定義する。現在は文章として存在するが、実行可能な判断基準がない。

## Current state

- `docs/compiler/bootstrap.md`: dual period end condition と deletion procedure が文章で記述
- 残す crate: ark-lsp, ark-dap, ark-diagnostics (IDE tooling)
- 消す crate: ark-driver, ark-mir, ark-wasm, arukellt (compiler pipeline)
- 実行順 / 各 crate の deletion 条件が明確でない
- `Cargo.toml` の default-members は全 crate を含む

## Acceptance

- [ ] retained crates リスト (IDE tooling 等) が確定し文書化される
- [ ] deletion procedure の実行順が numbered steps で定義される
- [ ] 各 compiler crate の deletion 条件: 「selfhost equivalent が parity check を pass」が明記される
- [ ] dual-period の終了判定基準が一意に定義される

## References

- `docs/compiler/bootstrap.md` — dual period / deletion procedure
- `Cargo.toml` — workspace members
