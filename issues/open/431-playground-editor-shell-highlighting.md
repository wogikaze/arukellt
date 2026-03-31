# Playground: editor shell と syntax highlighting を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 431
**Depends on**: 429
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 4

## Summary

Monaco または CodeMirror ベースの editor shell を作り、既存 syntax 資産を反映する。editor は playground の顔なので、最低限の highlighting、diagnostics markers、format action を揃える。

## Current state

- docs site は static shell のみで editor UI がない。
- syntax highlighting の source of truth は extension 側にあるが、browser 用再利用が未整理。
- editor と Wasm engine の接続点が無い。

## Acceptance

- [ ] browser editor が追加される。
- [ ] syntax highlighting が動作する。
- [ ] diagnostics markers または panel 表示が動作する。
- [ ] format action が editor から呼べる。

## References

- ``docs/index.html``
- ``extensions/arukellt-all-in-one/**``
- ``crates/ark-parser/src/fmt.rs``
