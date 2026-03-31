# check-docs-consistency の検証範囲を拡張する

**Status**: open
**Created**: 2026-03-31
**ID**: 302
**Depends on**: 301
**Track**: main
**Priority**: 22

## Summary

`check-docs-consistency.py` は `generate-docs.py --check` を呼ぶだけ。bootstrap 状態・capability 状態・component 状態の整合チェックが含まれていない。

## Current state

- `scripts/check-docs-consistency.py`: 22行の thin wrapper
- 生成 docs の新旧一致しか検証しない
- 手書き docs (bootstrap 節、known limitations) の stale 検出ができない

## Acceptance

- [ ] bootstrap 節の Stage 状態が `verify-bootstrap.sh --check` の結果と整合するか検証
- [ ] capability 状態が `std/manifest.toml` の `kind` 情報と整合するか検証
- [ ] component 状態が実装の対応型リストと整合するか検証
- [ ] stale 検出時に具体的な差分を出力する

## References

- `scripts/check-docs-consistency.py`
- `scripts/generate-docs.py`
- `scripts/verify-bootstrap.sh`
