# Language Docs: 安定した anchor / permalink 体系を整える

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 412
**Depends on**: —
**Track**: language-docs
**Blocks v1 exit**: no
**Priority**: 7

## Summary

spec と guide の節参照が長期的に壊れないよう、anchor 命名規約と permalink 方針を決める。docs の分割・再編後も、issue や external link がなるべく壊れない状態を目指す。

## Current state

- 長大な markdown の heading 変更でアンカーが変わりやすい。
- guide 追加後、同じ概念へのリンク先が複数になりやすい。
- external から引用される docs は stable permalink が望ましい。

## Acceptance

- [ ] anchor / permalink 規約が文書化される。
- [ ] 主要 docs に stable anchor が付与または維持される。
- [ ] 再編時の redirect または alias 方針が決まる。
- [ ] 最低限の link-check が追加される。

## References

- ``docs/language/**``
- ``docs/index.html``
- ``scripts/generate-docs.py``
