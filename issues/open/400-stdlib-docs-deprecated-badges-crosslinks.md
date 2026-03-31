# Stdlib Docs: deprecated API の表示と migration cross-link を自動化する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 400
**Depends on**: 367
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 6

## Summary

deprecated API を reference に載せたまま放置せず、明確な badge・置き換え先・削除時期・migration guide へのリンクを自動表示する。docs での移行導線を product として完成させる。

## Current state

- deprecated API の見え方が uniform ではない。
- reference と migration guide の接続が自動ではない。
- 削除時期や後継 API 情報を表示しにくい。

## Acceptance

- [ ] deprecated badge が自動表示される。
- [ ] 後継 API と migration guide へのリンクが自動表示される。
- [ ] 削除予定の metadata を表示できる。
- [ ] deprecated API を含む page で表示が一貫する。

## References

- ``docs/stdlib/reference.md``
- ``docs/stdlib/prelude-migration.md``
- ``std/manifest.toml``
- ``scripts/generate-docs.py``
