# Repo Hygiene: examples / fixtures / samples の source-of-truth map を作る

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 420
**Depends on**: —
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 4

## Summary

docs examples、fixtures、playground examples、sample code の関係を整理し、どれが正本でどれが派生なのかを明記する。これを作らないと docs と playground の両方で重複が増える。

## Current state

- `tests/fixtures/`、`docs/examples/`、sample 群、playground examples の将来計画が別々に見える。
- 同じコード片が複数場所で増える危険がある。
- source-backed にしたいが、正本の宣言がない。

## Acceptance

- [ ] example/fixture/sample の source-of-truth map が作成される。
- [ ] 各カテゴリの正本と派生先が定義される。
- [ ] 重複禁止または同期ルールが文書化される。
- [ ] docs または playground issue から参照できる。

## References

- ``tests/fixtures/**``
- ``docs/examples/**``
- ``docs/sample/**``
