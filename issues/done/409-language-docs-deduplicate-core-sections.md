---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 409
Track: language-docs
Depends on: 406, 408
Orchestration class: implementation-ready
---
# Language Docs: syntax / type / error / memory の重複記述を統合する
**Blocks v1 exit**: no
**Priority**: 4

## Summary

syntax/type/error/memory の説明が spec と guide と分割文書で過度に重複しないようにする。4 件に機械分割するのではなく、共通の source paragraph と補足文書への切り分けを実装する。

## Current state

- 同じ概念が `spec.md` と分割 docs の両方に詳しく書かれ、将来 drift しやすい。
- guide 追加後は重複がさらに増えるおそれがある。
- どこに canonical explanation を置くかが固定されていない。

## Acceptance

- [x] 重複が大きい節の inventory が作成される。
- [x] canonical explanation を置くページが節ごとに決まる。
- [x] 重複記述が削減され、参照リンクで接続される。
- [x] 少なくとも主要 4 領域で統合作業が行われる。

## References

- ``docs/language/spec.md``
- ``docs/language/syntax.md``
- ``docs/language/type-system.md``
- ``docs/language/error-handling.md``
- ``docs/language/memory-model.md``