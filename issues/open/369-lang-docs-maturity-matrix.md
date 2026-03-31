# Language Docs: feature maturity matrix を作成する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 369
**Depends on**: —
**Track**: language-docs
**Blocks v1 exit**: no
**Priority**: 6

## Summary

言語機能ごとの実装状態 (implemented / experimental / unimplemented) を一覧表として作成し、利用者が「この機能は今使えるか」を即座に判断できるようにする。spec.md 内の stability label を抽出し、独立した matrix 文書として維持する。

## Current state

- `docs/language/spec.md` に stability labels (stable, provisional, experimental, unimplemented) が散在
- labels を spec 全体を読まずに把握する手段がない
- `docs/current-state.md` に一部の feature status が記載されているが、言語機能全域をカバーしていない

## Acceptance

- [ ] 全言語機能の maturity matrix が `docs/language/` に存在する
- [ ] matrix が stable / provisional / experimental / unimplemented で分類される
- [ ] matrix が `spec.md` の labels と自動同期される (生成 or CI check)
- [ ] `docs/language/README.md` から matrix へのリンクがある

## References

- `docs/language/spec.md` — stability labels
- `docs/current-state.md` — feature status
