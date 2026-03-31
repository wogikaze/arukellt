# Stdlib: manifest metadata を resolver / typechecker / LSP / docs に伝搬する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 383
**Depends on**: —
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 1

## Summary

`std/manifest.toml` を単なる docs 用データではなく、コンパイラ・LSP・docs 生成の共通ソースとして扱う。kind / stability / target / aliases / deprecated metadata が各ツールに別々に埋め込まれており、stdlib surface の意味が場所ごとにずれている。まず metadata 伝搬の一本化を行い、後続の拡充や docs 整備の前提を作る。

## Current state

- `std/manifest.toml` には function ごとの signature / stability / target / kind があるが、resolver・LSP・docs がそれぞれ別の表現を持つ。
- `crates/ark-lsp/src/server.rs` には stdlib completion 候補の hardcoded 一覧が残っている。
- docs 側は `scripts/generate-docs.py` が manifest を参照する一方、compiler 側は metadata の一部しか利用していない。
- deprecated / alias / capability 表示の整合性が自動では保証されていない。

## Acceptance

- [ ] `std/manifest.toml` から取り出す共通 metadata 構造体が 1 箇所に定義される。
- [ ] resolver / LSP / docs generator が同じ metadata surface を参照する。
- [ ] hardcoded stdlib 名称一覧が削減または廃止される。
- [ ] metadata 不整合を検出するテストまたは CI チェックが追加される。

## References

- ``std/manifest.toml``
- ``crates/ark-stdlib/src/lib.rs``
- ``crates/ark-resolve/``
- ``crates/ark-lsp/src/server.rs``
- ``scripts/generate-docs.py``
