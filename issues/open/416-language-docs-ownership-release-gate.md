# Language Docs: ownership map と release gate を定義する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 416
**Depends on**: 413, 415
**Track**: language-docs
**Blocks v1 exit**: no
**Priority**: 11

## Summary

言語 docs を継続的に current に保つため、どの文書が generated、どの文書が curated、誰が責任を持つかを明示し、release 前チェックに組み込む。

## Current state

- 言語 docs は規模が大きく、個別ページの責務境界が曖昧になりやすい。
- guide / spec / archive / matrix の維持責任がページを跨いで分散している。
- release checklist での言語 docs quality gate が薄い。

## Acceptance

- [ ] language docs の ownership map が作成される。
- [ ] release gate に language docs 項目が追加される。
- [ ] generated / curated / archive の責務差が文書化される。
- [ ] quality gate が docs または CI と接続される。

## References

- ``docs/language/**``
- ``docs/release-checklist.md``
- ``scripts/generate-docs.py``
