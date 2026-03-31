# Playground: v1 の product contract と execution ADR を固定する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 428
**Depends on**: 378
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 1

## Summary

playground を作り始める前に、v1 が何を提供し何を提供しないかを ADR と product contract に落とす。edit / format / parse / diagnostics / examples / share を中心にし、フル実行や server dependency の扱いを明記する。

## Current state

- playground は未着手で、execution model が未確定。
- T2 target は未実装、T3 は browser-native ではないため、scope を誤ると実装が膨らむ。
- docs / examples / stdlib / repo hygiene と接続する前提が多い。

## Acceptance

- [ ] ADR が追加される。
- [ ] v1 scope と非目標が明文化される。
- [ ] client-side / server-side / hybrid の選択理由が記録される。
- [ ] docs / tests / examples との接続点が書かれる。

## References

- ``docs/adr/**``
- ``docs/target-contract.md``
- ``docs/current-state.md``
- ``crates/ark-target/src/lib.rs``
