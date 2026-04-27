---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 365
Track: stdlib-docs
Depends on: 363
Orchestration class: implementation-ready
---
# Stdlib Docs: source-backed な cookbook / recipe 集を拡充する
**Blocks v1 exit**: no
**Priority**: 15

## Summary

`docs/stdlib/cookbook.md` を、fixture-backed で検証可能な recipe 集として拡充する。各 recipe は `tests/fixtures/` の対応ファイルと紐づけ、CI で recipe コードが動作することを保証する。host capability を必要とする recipe と pure library recipe を区分する。

## Current state

- `docs/stdlib/cookbook.md` が存在するが、recipe の網羅性と source-backed 化が不十分
- `tests/fixtures/` に 588 fixture があるが、cookbook との紐づけが明示的でない
- `docs/examples/` に例コードがあるが、cookbook とは別管理

## Acceptance

- [x] 主要ユースケース (ファイル操作, JSON 処理, HTTP, テスト, コレクション操作) の recipe が存在する
- [x] 各 recipe に対応する fixture ファイルパスが明記される
- [x] recipe コードが CI (fixture harness) で検証される
- [x] host capability を必要とする recipe に target 制約が明記される

## References

- `docs/stdlib/cookbook.md` — cookbook
- `tests/fixtures/` — fixture files
- `docs/examples/` — example code