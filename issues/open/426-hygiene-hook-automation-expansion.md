# Repo Hygiene: pre-commit / pre-push の検証項目を cleanup 観点で拡張する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 426
**Depends on**: 376
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 10

## Summary

既存 hook に cleanup / freshness / naming / generated ownership の観点を追加する。人手レビューだけでは抜けやすい hygiene チェックを軽量自動化する。

## Current state

- hook は存在するが、主に fmt / docs consistency / harness 寄りで、cleanup 専用の観点は限定的。
- 新たな hygiene ルールを増やしても hook に入れないと運用されにくい。
- ローカルで落ちる方が安いチェックが CI に流れている。

## Acceptance

- [ ] 新しい hygiene チェックが pre-commit または pre-push に追加される。
- [ ] 追加されたチェックの一覧が文書化される。
- [ ] 失敗時の修正手順が案内される。
- [ ] CI と hook の責務分担が書かれる。

## References

- ``scripts/pre-commit-verify.sh``
- ``scripts/pre-push-verify.sh``
- ``scripts/verify-harness.sh``
