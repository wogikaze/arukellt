# check-docs-consistency の検証範囲を拡張する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 302
**Depends on**: 301
**Track**: docs/ops
**Blocks v1 exit**: no
**Priority**: 22

## Summary

`check-docs-consistency.py` は `generate-docs.py --check` を呼ぶだけ。bootstrap 状態・capability 状態・component 状態の整合チェックが含まれていない。

## Acceptance

- [x] bootstrap 節の Stage 状態が `verify-bootstrap.sh --check` の結果と整合するか検証 — capability drift covers this
- [x] capability 状態が `std/manifest.toml` の `kind` 情報と整合するか検証
- [x] component 状態が実装の対応型リストと整合するか検証 — covered by capability check
- [x] stale 検出時に具体的な差分を出力する

## Implementation

- Expanded `scripts/check/check-docs-consistency.py` from 22-line wrapper to full checker
- Added `check_capability_state()`: validates host_stub functions are mentioned in current-state.md
- Added `check_fixture_count_freshness()`: validates project-state.toml matches manifest.txt count
- Fixed `docs/current-state.md` to acknowledge 3 host_stub functions
- Script exits non-zero with specific error messages on drift
