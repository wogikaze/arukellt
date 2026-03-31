# Language Docs: historical / archive 文書の境界を明確化する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 372
**Depends on**: —
**Track**: language-docs
**Blocks v1 exit**: no
**Priority**: 13

## Summary

`docs/language/` 内の current 文書と historical / archive / transitional 文書の境界を明確にする。`docs/spec/` は既に archive と明示されているが、`docs/language/` 内部では `syntax-v1-preview.md` のような transitional 文書が current 文書と同列に並んでいる。

## Current state

- `docs/spec/README.md`: archive であることを明示 (良い状態)
- `docs/language/syntax-v1-preview.md`: preview / transitional だが位置づけが曖昧
- `docs/current-state.md`: historical docs が current reality を override してはならないと方針記載あり
- `docs/language/` 内部では current と transitional の区別が視覚的に弱い

## Acceptance

- [ ] `docs/language/` 内の全文書に current / transitional / archive のラベルが付与される
- [ ] archive / transitional 文書に banner (免責注記) が設置される
- [ ] `docs/language/README.md` の generated table にラベルが反映される
- [ ] `scripts/generate-docs.py` が archive banner を自動挿入する

## References

- `docs/language/syntax-v1-preview.md` — transitional
- `docs/spec/README.md` — archive 明示例
- `docs/current-state.md` — historical override 禁止方針
- `scripts/generate-docs.py` — docs 生成
