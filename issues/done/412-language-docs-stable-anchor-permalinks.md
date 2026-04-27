---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 412
Track: language-docs
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 7
# Language Docs: 安定した anchor / permalink 体系を整える
---
# Language Docs: 安定した anchor / permalink 体系を整える

## Summary

spec と guide の節参照が長期的に壊れないよう、anchor 命名規約と permalink 方針を決める。docs の分割・再編後も、issue や external link がなるべく壊れない状態を目指す。

## Current state

- 長大な markdown の heading 変更でアンカーが変わりやすい。
- guide 追加後、同じ概念へのリンク先が複数になりやすい。
- external から引用される docs は stable permalink が望ましい。

## Acceptance

- [x] anchor / permalink 規約が文書化される。
- [x] 主要 docs に stable anchor が付与または維持される。
- [x] 再編時の redirect または alias 方針が決まる。
- [x] 最低限の link-check が追加される。

## References

- ``docs/language/**``
- ``docs/index.html``
- ``scripts/gen/generate-docs.py``