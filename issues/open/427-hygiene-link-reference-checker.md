# Repo Hygiene: broken link / missing file reference checker を追加する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 427
**Depends on**: 426
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 11

## Summary

docs、issues、README、generated index 間のリンク切れや存在しないファイル参照を検出する。cleanup はファイル削除だけでなく、残った参照の健全性も含む。

## Current state

- docs と issues は相互参照が多く、ファイル移動や archive 化でリンクが切れやすい。
- generator はリンク先の存在まで網羅的に検証していない。
- cleanup 後の欠損参照を見落としやすい。

## Acceptance

- [ ] リンク / 参照チェッカーが追加される。
- [ ] docs と issues の少なくとも主要参照を検証する。
- [ ] 欠損時に具体的な file/path が出る。
- [ ] CI または hook で実行される。

## References

- ``docs/**``
- ``issues/**``
- ``README.md``
- ``scripts/**``
