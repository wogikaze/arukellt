# Component Composition: 複数コンポーネントの合成と linking モデルを定義・実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 443
**Depends on**: 442
**Track**: component-composition
**Blocks v1 exit**: no
**Priority**: 5

## Summary

複数の Wasm Component を合成し、依存関係を解決して実行可能な構成を作る linking モデルを導入する。package system / dependency graph と連動させる。

## Current state

- 単体 component 出力のみ。
- linking / composition 機構なし。
- package-level component 概念なし。

## Acceptance

- [ ] component 同士の import/export を解決可能にする。
- [ ] 複数 component を 1つの実行単位に合成できる。
- [ ] dependency graph が構築される。
- [ ] conflict（名前/型）の検出が可能。
- [ ] CLI から compose/build が実行可能。

## References

- `crates/ark-driver/`
- `crates/ark-wasm/`
- `docs/target-contract.md`
- `docs/ark-toml.md`
