# Repo Hygiene: benchmark / test asset の命名規約を統一する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 374
**Depends on**: —
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 16

## Summary

`benchmarks/` と `tests/` 配下のアセット命名の揺れ (kebab-case / snake_case / mixed) を統一し、canonical naming policy を定める。命名規約がないまま asset が増えると、playground や docs examples との連携時に混乱する。

## Current state

- `benchmarks/vec-ops.*` と `benchmarks/vec_ops.*` のように命名が揺れている可能性
- `tests/fixtures/` は 588 件あり、大半は snake_case だが例外があるかもしれない
- naming policy が明文化されていない

## Acceptance

- [x] benchmark / test asset の命名規約 (kebab-case or snake_case) が文書化される
- [x] 既存 asset が規約に準拠するようリネームされる
- [x] CI が命名規約違反を検出する (or pre-commit check)
- [x] `tests/fixtures/manifest.txt` が命名規約に準拠する

## References

- `benchmarks/` — benchmark assets
- `tests/fixtures/` — test fixtures
- `tests/fixtures/manifest.txt` — fixture manifest
