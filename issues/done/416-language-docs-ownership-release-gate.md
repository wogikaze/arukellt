---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 416
Track: language-docs
Depends on: 413, 415
Orchestration class: implementation-ready
---
# Language Docs: ownership map と release gate を定義する
**Blocks v1 exit**: no
**Priority**: 11

## Summary

言語 docs を継続的に current に保つため、どの文書が generated、どの文書が curated、誰が責任を持つかを明示し、release 前チェックに組み込む。

## Current state

- 言語 docs は規模が大きく、個別ページの責務境界が曖昧になりやすい。
- guide / spec / archive / matrix の維持責任がページを跨いで分散している。
- release checklist での言語 docs quality gate が薄い。

## Acceptance

- [x] language docs の ownership map が作成される。
- [x] release gate に language docs 項目が追加される。
- [x] generated / curated / archive の責務差が文書化される。
- [x] quality gate が docs または CI と接続される。

## References

- ``docs/language/**``
- ``docs/release-checklist.md``
- ``scripts/gen/generate-docs.py``