---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 406
Track: language-docs
Depends on: 368
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 1
# Language Docs: current-first guide の骨格と章立てを作る
---
# Language Docs: current-first guide の骨格と章立てを作る

## Summary

大きな spec とは別に、日常利用者向けの current-first guide を実際に起こす。構文、型、エラー、モジュール、target 制約の最小セットを stable/implemented だけで説明する骨格を先に作る。

## Current state

- `docs/language/spec.md` は規範文書として強いが、学習入口としては重い。
- current-first を意図した文書は散在しているが、一本の guide としては構成されていない。
- 利用者が「今何を書けるか」だけを読みたいときの入口が薄い。

## Acceptance

- [x] current-first guide ファイルが追加される。
- [x] 章立てが stable/implemented な内容だけで構成される。
- [x] `docs/language/README.md` から guide へ導線が張られる。
- [x] spec と guide の責務差が冒頭で明記される。

## References

- ``docs/language/spec.md``
- ``docs/language/README.md``
- ``docs/language/syntax.md``
- ``docs/language/type-system.md``