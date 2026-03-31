# Repo Hygiene: scaffold / internal コンポーネントの露出 tier を README と docs に反映する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 423
**Depends on**: 377
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 7

## Summary

露出 tier の方針を実際の README / current-state / workspace overview に反映する。利用者向け surface と内部資産が同じ強さで並ばないようにする rollout issue。

## Current state

- internal / scaffold コンポーネントが current product surface と同列に見える箇所がある。
- tier は方針として議論できても、README 等の表示にはまだ落ちていない。
- 新規参加者がどれを使うべきか迷いやすい。

## Acceptance

- [ ] README の component 紹介が tier に沿って更新される。
- [ ] current-state の表記も tier に揃う。
- [ ] internal/scaffold に説明注記が付く。
- [ ] 少なくとも主要露出箇所が一通り更新される。

## References

- ``README.md``
- ``docs/current-state.md``
- ``Cargo.toml``
- ``crates/**``
