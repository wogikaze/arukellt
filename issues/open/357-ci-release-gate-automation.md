# Tooling Contract: release gate を手動チェックリストから自動検証に移行する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 357
**Depends on**: 354, 355, 356
**Track**: tooling-contract
**Blocks v1 exit**: no
**Priority**: 25

## Summary

`docs/release-checklist.md` の手動チェック項目のうち、extension / LSP / formatter / linter 関連を CI job に置き換え、release gate を自動化する。手動確認は「自動化不可能な項目のみ」に縮小する。

## Current state

- `docs/release-checklist.md`: 全 extension 検証が manual (10 項目)
- LSP の動作確認も manual
- formatter / linter の一貫性確認も manual
- CI には extension test job がない (#354)

## Acceptance

- [ ] release-checklist の extension 検証項目のうち自動化可能なものが CI job に移行される
- [ ] LSP protocol compliance が CI で検証される (#355 前提)
- [ ] formatter / CLI / LSP の出力一致が CI で検証される (#356 前提)
- [ ] `docs/release-checklist.md` が更新され、残る手動項目が明示される

## References

- `docs/release-checklist.md` — release checklist
- `.github/workflows/ci.yml` — CI 定義
