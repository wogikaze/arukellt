# Language Docs: normative / explanatory / transitional の境界を ADR と banner で固定する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 408
**Depends on**: —
**Track**: language-docs
**Blocks v1 exit**: no
**Priority**: 3

## Summary

言語 docs を読むときに、何が規範文書で何が説明文書かを常に判別できるようにする。分類を ADR に落とし、各ページに対応する banner を付ける。

## Current state

- `docs/language` 内で current-first と frozen spec と preview が混在して見える。
- 読者が文書の「法的強さ」をページごとに推測する必要がある。
- archive と transitional の扱いが directory に依存しており、page 自体の注記が弱い。

## Acceptance

- [x] 文書分類の ADR が追加される。
- [x] 分類に応じた banner テンプレートが定義される。
- [x] 主要 language docs に banner が付与される。
- [x] README の table に分類が反映される。

## References

- ``docs/language/**``
- ``docs/spec/README.md``
- ``docs/adr/**``
