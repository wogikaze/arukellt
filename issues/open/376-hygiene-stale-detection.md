# Repo Hygiene: stale state を CI / pre-push で自動検出する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 376
**Depends on**: 373
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 19

## Summary

generated docs の再生成漏れ、manifest と実装の不整合、fixture manifest の不完全性、issue index の stale 状態を CI / pre-push で自動検出する仕組みを強化する。現在 `scripts/check-docs-consistency.py` と `scripts/pre-push-verify.sh` が一部をカバーしているが、検出範囲を拡張する。

## Current state

- `scripts/check-docs-consistency.py`: docs 整合性チェック (一部)
- `scripts/pre-push-verify.sh`: pre-push hook で 17 項目チェック
- `scripts/pre-commit-verify.sh`: pre-commit hook で fmt / markdownlint / docs consistency チェック
- generated ファイルの再生成漏れ検出が完全ではない
- issue index の stale 検出なし

## Acceptance

- [ ] generated docs の再生成漏れが CI で検出される
- [ ] issue index が stale の場合 CI が警告する
- [ ] `std/manifest.toml` と実装ソースの不整合が CI で検出される
- [ ] 新規 stale 検出項目が `scripts/pre-push-verify.sh` に追加される

## References

- `scripts/check-docs-consistency.py` — 整合性チェック
- `scripts/pre-push-verify.sh` — pre-push hook
- `scripts/pre-commit-verify.sh` — pre-commit hook
