# blocked issue の upstream 状態を確認・更新する

**Status**: open
**Created**: 2026-03-31
**ID**: 305
**Depends on**: —
**Track**: main
**Priority**: 25

## Summary

`issues/blocked/037-jco-gc-support.md` が upstream の jco リポジトリの現在の状態と一致しているか確認し、必要に応じて更新する。

## Current state

- `issues/blocked/037`: jco v1.16.1/v1.17.5 が Wasm GC components を transpile できない
- jco upstream の進捗を最後に確認した日付が不明
- unblock 条件が 2 つ記載されている

## Acceptance

- [ ] jco upstream の最新リリースで GC 対応状況を確認
- [ ] 確認日付を issue に記録
- [ ] unblock 条件の進捗を更新
- [ ] 解除可能なら `issues/blocked/` → `issues/open/` に移動

## References

- `issues/blocked/037-jco-gc-support.md`
- <https://github.com/bytecodealliance/jco>
