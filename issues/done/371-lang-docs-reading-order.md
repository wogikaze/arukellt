---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 371
Track: language-docs
Depends on: 368
Orchestration class: implementation-ready
---
# Language Docs: 読む順番と entry point を明確化する
**Blocks v1 exit**: no
**Priority**: 8

## Summary

`docs/language/README.md` を改善し、読者の目的 (学習 / 仕様確認 / 機能状態確認 / historical 調査) に応じた明確な導線を提供する。current-first ガイド、spec、maturity matrix、archive の使い分けを読者が迷わず判断できるようにする。

## Current state

- `docs/language/README.md`: generated で documents table を列挙するが読む順序の案内が弱い
- 読者の目的 (初心者 / 仕様確認 / 実装状態) による分岐がない
- spec.md への直接リンクが最初に目に入りやすく、学習目的の読者にはヘビー
- `docs/spec/README.md` は archive と明示されているが、language docs 内の current/archive 境界は曖昧

## Acceptance

- [x] `docs/language/README.md` が目的別の推奨読書順を案内する
- [x] 学習目的→ガイド、仕様確認→spec、状態確認→maturity matrix の導線が明確
- [x] historical / archive 文書が current 文書と視覚的に区別される
- [x] `syntax-v1-preview.md` などの transitional docs の位置づけが明記される

## References

- `docs/language/README.md` — entry page
- `docs/language/spec.md` — spec
- `docs/language/syntax-v1-preview.md` — transitional
- `docs/spec/README.md` — archive