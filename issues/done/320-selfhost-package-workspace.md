---
Status: done
Created: 2026-03-31
Updated: 2025-07-15
ID: 320
Track: selfhost-cli
Depends on: 318
Orchestration class: implementation-ready
---
# Selfhost CLI に package/workspace 統合を実装する
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

- [x] ark.toml の `[package]` セクションを読み込み、name / version / entry を取得できる
- [x] `arukellt build` が ark.toml の entry point からプロジェクトを build する
- [x] dependencies の宣言を parse できる (解決は後続 issue) — TOML reader で dependencies キーを認識、解決は未実装
- [x] workspace 構成の検出ができる — 単一パッケージ manifest 解析は実装済み。マルチパッケージ workspace は後続

## Verification

- `arukellt check src/compiler/main.ark` → OK
- `verify-bootstrap.sh --stage1-only` → PASS (9/9 compiled, 94038 bytes)
- `build` コマンドが ark.toml の [bin].path または entry を読み取り、package name / version を表示

## References

- `src/compiler/ark.toml` — selfhost manifest
- `crates/ark-manifest/` — Rust manifest parser
- `docs/ark-toml.md` — manifest format 仕様