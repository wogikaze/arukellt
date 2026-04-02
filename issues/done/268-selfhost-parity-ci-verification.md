# Stage1 fixture parity・CLI parity・diagnostic parity を CI で継続検証する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 268
**Depends on**: 267
**Track**: main
**Blocks v1 exit**: yes

## Summary

Rust 実装と selfhost 実装の parity（fixture 結果・CLI 出力・診断メッセージ）が日次で崩れていないことを示す CI 契約が存在しない。この issue では parity 検証を CI ジョブとして配線する。

## Acceptance

- [x] CI に `selfhost-parity` ジョブが存在し、Rust 実装と selfhost 実装の両方で同一 fixture を実行して結果を比較する
- [x] `CLI parity`（同一入力に対して同一 stdout/stderr）が検証されている
- [x] `diagnostic parity`（エラーメッセージの内容・位置情報）が検証されている
- [x] parity 乖離が検出された場合に diff が CI ログに出力される

## Scope

- `scripts/check/check-selfhost-parity.sh`（または同等スクリプト）の実装
- Rust 実装と selfhost 実装の出力を比較する fixture セットの定義
- CI ジョブへの組み込み

## References

- `scripts/run/verify-bootstrap.sh`
- `issues/open/267-verify-bootstrap-upgrade.md`
- `issues/open/253-selfhost-completion-criteria.md`
