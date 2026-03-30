# package/workspace/manifest resolution テストを第一級化する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 263
**Depends on**: 261
**Track**: main
**Blocks v1 exit**: yes

## Summary

`ark.toml` / workspace / script 実行 / manifest resolution / diagnostics snapshots のテストは「存在する」ことと「壊れたら即検出される」ことが別の状態になっている。これらを package-workspace カテゴリとして第一級化する。

## Acceptance

- [ ] `tests/package-workspace/` 以下にテスト fixture が整備されている
- [ ] `ark.toml` の基本フィールド（name / version / target / dependencies）のバリデーションがテストされている
- [ ] workspace（複数 package）の解決がテストされている
- [ ] manifest resolution 失敗時の診断メッセージがスナップショットテストで保護されている
- [ ] CI でこれらが独立した step として実行される

## Scope

- `tests/package-workspace/` ディレクトリと fixture の作成
- `ark.toml` parse・validate の単体テストの追加または強化
- workspace 解決の integration test の追加
- diagnostics snapshot test の仕組みを確立

## References

- `issues/open/231-ark-toml-as-project-model-entry-point.md`
- `issues/open/233-module-package-workspace-resolution-spec.md`
- `issues/open/261-test-category-classification-scheme.md`
- `issues/open/252-test-strategy-overhaul.md`
