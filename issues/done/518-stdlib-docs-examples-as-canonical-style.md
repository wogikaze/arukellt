# Stdlib: docs / fixtures / cookbook を「良い Arukellt コード例」として再監査する

**Status**: done
**Created**: 2026-04-15
**Updated**: 2026-04-22
**Closed**: 2026-04-22
**Commit**: b15af079
**ID**: 518
**Depends on**: 513, 517
**Track**: stdlib
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #513, #517
**Blocks v{N}**: none
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib docs と fixture examples は単なる動作証明ではなく、ユーザーが真似する reference style でもある。
古い prelude 直叩き、数値タグ前提、raw helper 優先、曖昧な error handling を含む example を洗い出し、
module-qualified / typed / facade-first な style に更新する。

## Repo evidence

- `docs/cookbook/wasm-binary.md` の WIT example は numeric ID 前提だった
- `docs/stdlib/modules/wit.md` は integer constants 中心の説明だった
- fixture examples は historical API usage を保持していることがある

## Acceptance

- [ ] docs / cookbook / stdlib fixtures の「style debt」一覧が作成される
- [ ] user-facing example で避けるべき anti-pattern が定義される
- [ ] 少なくとも `std::wit`, `std::json`, `std::bytes`, `std::io` family の examples 更新計画がある
- [ ] compile-check 対象 examples と migration 対象 examples が区別される

## Primary paths

- `docs/stdlib/`
- `docs/cookbook/`
- `tests/fixtures/stdlib_*`
- `std/manifest.toml`

## References

- `issues/done/365-stdlib-docs-cookbook.md`
- `issues/done/401-stdlib-docs-example-compile-check.md`
