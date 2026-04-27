---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 368
Track: language-docs
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 2
---

# Language Docs: current-first な言語ガイドを spec から分離して作る
- `docs/language/spec.md`: 1300 行超、stable / provisional / experimental / unimplemented が混在
- `docs/language/syntax.md` / `type-system.md` / `error-handling.md`: current-first と明記されているが spec との重複あり
# Language Docs: current-first な言語ガイドを spec から分離して作る

## Summary

`docs/language/spec.md` (1300 行超) は authoritative な規範文書として維持しつつ、日常利用向けの current-first ガイドを別文書として作成する。ガイドは「今書ける構文」「今使える型」「今動く機能」だけを扱い、unimplemented / experimental を含まない。利用者が spec を全部読まなくても言語を学べる導線を作る。

## Current state

- `docs/language/spec.md`: 1300 行超、stable / provisional / experimental / unimplemented が混在
- `docs/language/syntax.md` / `type-system.md` / `error-handling.md`: current-first と明記されているが spec との重複あり
- 利用者が「今書ける構文だけを知りたい」場合の明確な入口がない
- `docs/language/README.md` は generated で entry を列挙するが読む順序の案内が弱い

## Acceptance

- [x] current-first ガイド文書が `docs/language/` に存在する
- [x] ガイドが stable / implemented な機能のみを扱い、experimental / unimplemented を含まない
- [x] `docs/language/README.md` がガイドを最初の推奨読書先として案内する
- [x] spec.md との責務分担が文書化される (spec = normative + all tiers, guide = current + stable only)

## References

- `docs/language/spec.md` — 規範仕様
- `docs/language/syntax.md` — current-first 構文
- `docs/language/type-system.md` — current-first 型
- `docs/language/README.md` — entry page