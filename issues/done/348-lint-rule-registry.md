---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 348
Track: linter
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 13
---

# Linter: lint rule registry を作る
- `crates/ark-diagnostics/src/codes.rs`: warning コードは W0001-W0005 の 5 個のみ、全て compiler 内部由来
# Linter: lint rule registry を作る

## Summary

compiler の hard error / semantic warning とは独立した lint rule registry を設計・実装する。各 rule に ID / severity / category / description / fix-it 有無を持たせ、`check` の副産物ではない first-class な lint 基盤を作る。

## Current state

- `crates/ark-diagnostics/src/codes.rs`: warning コードは W0001-W0005 の 5 個のみ、全て compiler 内部由来
- lint rule を登録・列挙・管理する仕組みがない
- category (style / correctness / performance / migration) の分類がない
- rule ごとの allow / warn / deny 設定がない

## Acceptance

- [x] `LintRule` trait/struct が定義され、id / severity / category / description を持つ
- [x] rule registry が存在し、全 rule を列挙できる
- [x] 既存 W0001-W0005 が registry 上の rule として登録される
- [x] severity が Warning / Allow / Deny の 3 段階で設定可能
- [x] `arukellt check` が registry の rule を走らせる (既存動作は維持)

## References

- `crates/ark-diagnostics/src/codes.rs` — 既存 warning コード
- `crates/ark-diagnostics/src/helpers.rs` — diagnostic 生成ヘルパー