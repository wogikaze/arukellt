# Playground: unsupported target / host capability を明示する UX を入れる

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 435
**Depends on**: 428
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 8

## Summary

playground v1 では実行できない API や target があるため、エラーではなく product-level guidance として見せる。host API、unsupported target、needs-wasi な examples を明示する。

## Current state

- browser playground では host capability を伴うコードがそのままは動かない。
- unsupported 状態を無説明で失敗させると体験が悪い。
- docs / examples で capability 情報を共有したい。

## Acceptance

- [x] unsupported API / target に対する UX が実装される。
- [x] examples catalog と連携して capability 注意が表示される。
- [x] ユーザーが「なぜ動かないか」を見て分かる。
- [x] 少なくとも host API 使用コードで警告表示が出る。

## References

- ``std/manifest.toml``
- ``docs/target-contract.md``
- ``docs/stdlib/reference.md``
