# Stdlib: error / Result の命名・変換・伝播慣習を正規化する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 392
**Depends on**: —
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 10

## Summary

stdlib 全体で error / result の命名や返し方が揺れないように整える。`parse_*`、`try_*`、`*_or`、`*_or_else` のような慣習、error 型の粒度、host family と pure family の違いを整理し、LSP hover や docs が一貫した説明を出せるようにする。

## Current state

- family ごとに error message と result 返却慣習が微妙に異なる。
- host family と pure family で error の意味が違うが docs では並列に見えやすい。
- 移行時にどの関数名を canonical にするかが曖昧な箇所がある。

## Acceptance

- [ ] stdlib の error / Result naming convention が文書化される。
- [ ] 少なくとも複数 family で関数名または docs が正規化される。
- [ ] error 変換や伝播を確認する fixture が追加される。
- [ ] reference docs が慣習に沿った表記へ更新される。

## References

- ``std/**``
- ``docs/stdlib/reference.md``
- ``docs/stdlib/stability-policy.md``
- ``tests/fixtures/``
