# Language Docs: spec と guide の drift checker を CI に入れる

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 415
**Depends on**: 406, 407
**Track**: language-docs
**Blocks v1 exit**: no
**Priority**: 10

## Summary

stable/implemented な機能が guide に反映されているかを自動チェックする。人手レビューだけに依存せず、spec の安定機能変更時に guide 更新漏れを見つける。

## Current state

- spec と分割 docs の同期は手動で、更新漏れが起こりうる。
- guide を導入すると drift 面積が増える。
- CI は言語 docs の意味整合をまだ見ていない。

## Acceptance

- [x] spec-guide drift checker が追加される。
- [x] stable 機能の更新漏れを検出できる。
- [x] CI で結果が見える。
- [x] 失敗時にどの節を更新すべきかが分かる出力がある。

## References

- ``docs/language/spec.md``
- ``docs/language/**``
- ``scripts/check/check-docs-consistency.py``
