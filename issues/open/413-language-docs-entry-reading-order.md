# Language Docs: entry page に目的別 reading order を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 413
**Depends on**: 406
**Track**: language-docs
**Blocks v1 exit**: no
**Priority**: 8

## Summary

言語 docs の入口を「全部並べる」から「目的別に読む順番を示す」へ変える。学習、仕様確認、機能状態確認、歴史資料調査の 4 方向を明示し、入口で迷わせない。

## Current state

- `docs/language/README.md` は generated table が中心で、初心者が何から読むか分かりにくい。
- spec と guide の両方が目立つため、用途に応じた選び方が弱い。
- archive 文書との区別も README 上ではまだ薄い。

## Acceptance

- [ ] README に目的別の reading order が追加される。
- [ ] guide / spec / matrix / archive の導線が明確になる。
- [ ] table または badges に文書分類が反映される。
- [ ] generated 更新で崩れない構造になる。

## References

- ``docs/language/README.md``
- ``docs/spec/README.md``
- ``scripts/generate-docs.py``
