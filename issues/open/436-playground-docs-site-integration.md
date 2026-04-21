
## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: docs/playground/index.html exists but playground is non-functional without wasm binary; integration is structural only
- **Root cause**: The playground wasm binary (ark-playground-wasm) has never been compiled. crates/ark-playground-wasm/pkg/ does not exist. docs/playground/wasm/ is empty. All playground user-visible functionality depends on this binary.
- **Evidence**: `find . -name '*.wasm' -path '*playground*'` returns nothing; `ls crates/ark-playground-wasm/pkg/` fails; `ls docs/playground/wasm/` is empty.

# Playground: docs site への統合と navigation を実装する

**Status**: done
**Created**: 2026-04-03
**Updated**: 2026-04-21
**ID**: 436
**Depends on**: 437, 438, 464
**Track**: playground
**Orchestration class**: verification-ready
**Orchestration upstream**: —
**Blocks v4 exit**: no

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

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
