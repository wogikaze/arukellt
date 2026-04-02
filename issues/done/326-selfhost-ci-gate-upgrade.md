# CI の selfhost gate を full fixpoint check に昇格する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 326
**Depends on**: 325
**Track**: selfhost-verification
**Blocks v1 exit**: no
**Priority**: 20

## Summary

CI の `selfhost-stage0` job を full fixpoint check に昇格する。現在は Stage 0 (Rust が selfhost .ark を compile) のみ実行され、名前が misleading。

## Current state

- `.github/workflows/ci.yml`: `selfhost-stage0` job が `verify-bootstrap.sh --stage1-only` を実行
- 実態は Stage 0 only (Rust compilation のみ)
- Stage 1/2 fixpoint check は CI で実行されない
- parity check も CI で実行されない

## Acceptance

- [x] CI job が Stage 0 + Stage 1 + Stage 2 を実行する
- [x] fixpoint failure が merge-blocking になる
- [x] parity check の結果が CI output に含まれる
- [x] job 名が実態を反映する (`selfhost-fixpoint` 等)

## References

- `.github/workflows/ci.yml:328-363` — selfhost-stage0 job
- `scripts/verify-bootstrap.sh` — bootstrap verification
