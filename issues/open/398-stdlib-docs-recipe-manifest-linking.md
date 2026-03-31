# Stdlib Docs: recipe と fixtures / examples を結ぶ manifest を作る

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 398
**Depends on**: 365
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 4

## Summary

cookbook と tests / examples を二重管理しないため、recipe ごとの source-of-truth を持つ manifest を導入する。どの recipe がどの fixture・example・target 制約に対応するかを 1 箇所で管理する。

## Current state

- cookbook と fixture の対応が暗黙で、継続的に保守しにくい。
- examples と recipes が別系統で増えると重複しやすい。
- host capability を必要とする recipe の分類が page レベルで固定されていない。

## Acceptance

- [ ] recipe-manifest が追加される。
- [ ] 各 recipe が fixture または example に紐づく。
- [ ] target / capability 情報が manifest に入る。
- [ ] generator または CI がリンク切れを検出する。

## References

- ``docs/stdlib/cookbook.md``
- ``tests/fixtures/``
- ``docs/examples/``
- ``scripts/generate-docs.py``
