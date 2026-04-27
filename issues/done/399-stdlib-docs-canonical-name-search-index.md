---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 399
Track: stdlib-docs
Depends on: 395
Orchestration class: implementation-ready
---
# Stdlib Docs: canonical name / alias / historical name を横断検索できるようにする
**Blocks v1 exit**: no
**Priority**: 5

## Summary

deprecated や alias を含む stdlib 名称の揺れを docs 側で吸収する。canonical name、historical name、alias を検索索引として持ち、ユーザーが古い名前で検索しても新しいページへ辿り着ける状態にする。

## Current state

- monomorphic/historical name と canonical name の対応は reference を読まないと追いにくい。
- 検索導線が page title 依存になりやすく、古い名前で辿りにくい。
- deprecated docs と migration guide の接続が弱い。

## Acceptance

- [x] canonical / alias / historical name を持つ検索索引が生成される。
- [x] 古い名前から current page へ遷移できる。
- [x] deprecated ページまたは reference 行に migration guide リンクが出る。
- [x] 索引生成のテストがある。

## References

- ``docs/stdlib/reference.md``
- ``std/manifest.toml``
- ``docs/stdlib/prelude-migration.md``
- ``scripts/gen/generate-docs.py``