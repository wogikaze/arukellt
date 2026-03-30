# component interop テストを回帰面として整備する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 262
**Depends on**: 261
**Track**: main
**Blocks v1 exit**: yes

## Summary

`tests/component-interop/jco/calculator/run.sh` は単発 smoke に近く、export surface の広がりに対して十分な回帰面になっていない。component interop を第一級のテスト種別として整備する。

## Acceptance

- [ ] component interop テストが `tests/component-interop/` 以下に複数 fixture として整備されている
- [ ] 各 fixture が WIT export surface の代表的なパターン（primitive / record / variant / resource）を少なくとも1件ずつカバーしている
- [ ] `scripts/verify-harness.sh --component` がこれらの fixture を全件実行する
- [ ] CI で component interop が独立した step として実行される

## Scope

- `tests/component-interop/` に新規 fixture を追加（primitive 型・record・variant・resource の各パターン）
- 各 fixture の `run.sh` を標準化して `verify-harness.sh --component` から呼べるようにする
- CI の component interop step を追加

## References

- `tests/component-interop/`
- `issues/open/038-wit-type-fixtures.md`
- `issues/open/261-test-category-classification-scheme.md`
- `issues/open/252-test-strategy-overhaul.md`
