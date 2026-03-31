# Legacy lowering path を隔離・撤去する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 285
**Depends on**: 284
**Track**: corehir
**Blocks v1 exit**: no
**Priority**: 5

## Summary

CoreHIR がデフォルトになった後、legacy path (`lower_to_mir` in `func.rs`) を deprecated にマークし、fallback 経路を除去する。二重メンテナンスを終わらせる。

## Current state

- `crates/ark-mir/src/lower/func.rs:83-200+`: legacy lowering のメインエントリ
- `crates/ark-mir/src/lower/mod.rs:337-378`: `lower_corehir_with_fallback` が legacy にフォールバック
- CoreHIR path と legacy path の両方が常にメンテナンス対象

## Acceptance

- [ ] `lower_to_mir()` に `#[deprecated]` マークを付与
- [ ] `lower_corehir_with_fallback` のフォールバック経路を除去
- [ ] `--mir-select legacy` 使用時に deprecation warning を出す（1 リリース後に除去）
- [ ] 全 fixture が legacy なしで pass する

## References

- `crates/ark-mir/src/lower/func.rs`
- `crates/ark-mir/src/lower/mod.rs`
