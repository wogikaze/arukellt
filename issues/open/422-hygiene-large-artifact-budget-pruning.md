# Repo Hygiene: 大きな artifact と baseline の size budget / pruning ルールを作る

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 422
**Depends on**: —
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 6

## Summary

zip、baseline、generated artifact、sample 出力などの大型ファイルに budget と pruning ルールを持たせる。cleanup を「気付いたら減らす」運用から脱却する。

## Current state

- 大きなファイルが repo に残る理由と上限が明文化されていない。
- baseline や sample artifact は便利だが、増えるほど clone / review コストが上がる。
- size regression を検知する仕組みがない。

## Acceptance

- [ ] 大型 artifact の予算または許容ルールが文書化される。
- [ ] サイズ計測スクリプトまたはチェックが追加される。
- [ ] pruning の対象と残す理由の書式が決まる。
- [ ] 少なくとも 1 つの CI / hook でサイズ情報が見える。

## References

- ``tests/baselines/**``
- ``benchmarks/**``
- ``docs/**``
- ``scripts/**``
