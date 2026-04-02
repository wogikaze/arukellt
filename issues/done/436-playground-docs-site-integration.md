# Playground: docs site への統合と navigation を実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 436
**Depends on**: 431
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 9

## Summary

playground を独立ページで終わらせず、docs site から自然に辿れるようにする。examples や language/stdlib docs と行き来できる navigation を作る。

## Current state

- docs site と playground は分離されているどころか、playground 自体がない。
- docs examples から playground を開きたい需要がある。
- navigation を決めないと hidden feature になりやすい。

## Acceptance

- [x] docs site から playground への入口が追加される。
- [x] language / stdlib docs から example を playground で開ける導線がある。
- [x] playground から docs へ戻る導線がある。
- [x] site navigation に統合される。

## References

- ``docs/index.html``
- ``docs/examples/**``
- ``docs/stdlib/**``
- ``docs/language/**``
