# v5 Bootstrap: harness and CI integration

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-03-30
**ID**: 182
**Depends on**: 181
**Track**: main
**Blocks v1 exit**: no

## Summary

bootstrap verification を `verify-harness` と CI に接続する。ローカル比較スクリプトだけでなく、継続検証可能な状態にするための子 issue。

## Acceptance

- [x] bootstrap verification が harness / CI から実行できる
- [x] 条件付き実行や記録の残し方が追跡できる
- [x] docs (#169) が参照する verification entrypoint が定まっている

## References

- `issues/open/166-v5-bootstrap-verification.md`
- `issues/open/169-v5-bootstrap-doc.md`
- `scripts/run/verify-harness.sh`
