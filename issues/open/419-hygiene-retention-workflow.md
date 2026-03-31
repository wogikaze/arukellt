# Repo Hygiene: done issues / archive docs の retention workflow を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 419
**Depends on**: 375
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 3

## Summary

retention policy を文章だけでなく実運用手順に落とす。移動時の banner、日付、理由、index 更新の流れを決め、archive を増やしても current surface が濁らないようにする。

## Current state

- retention policy が明文化されていないため、done/ archive の扱いが ad hoc になりやすい。
- 移動時の記録や index 更新が手作業依存。
- historical と current の区別が directory 構造だけに依存する。

## Acceptance

- [ ] archive / done 移動手順が文書化される。
- [ ] 移動時に記録する metadata が決まる。
- [ ] 少なくとも issue または docs で運用例が整備される。
- [ ] 関連 index の更新手順が明記される。

## References

- ``issues/done/**``
- ``docs/spec/**``
- ``docs/README.md``
- ``docs/adr/**``
