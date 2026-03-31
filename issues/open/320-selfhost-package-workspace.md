# Selfhost CLI に package/workspace 統合を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 320
**Depends on**: 318
**Track**: selfhost-cli
**Blocks v1 exit**: no
**Priority**: 15

## Summary

ark.toml の読み込みと workspace build を selfhost CLI に実装する。Rust 版は manifest-driven build を持つが、selfhost は単一ファイル compile のみ。

## Current state

- `src/compiler/ark.toml` (307 bytes): selfhost 自身の manifest は存在する
- selfhost CLI は ark.toml を読み込む機能を持たない
- `crates/ark-manifest/` (Rust 側) が manifest parsing を担当
- workspace の複数パッケージ build は Rust CLI のみ

## Acceptance

- [ ] ark.toml の `[package]` セクションを読み込み、name / version / entry を取得できる
- [ ] `arukellt build` が ark.toml の entry point からプロジェクトを build する
- [ ] dependencies の宣言を parse できる (解決は後続 issue)
- [ ] workspace 構成の検出ができる

## References

- `src/compiler/ark.toml` — selfhost manifest
- `crates/ark-manifest/` — Rust manifest parser
- `docs/ark-toml.md` — manifest format 仕様
