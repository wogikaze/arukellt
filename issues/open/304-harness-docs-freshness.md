# verify-harness.sh に docs freshness check を追加する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 304
**Depends on**: 302
**Track**: docs/ops
**Blocks v1 exit**: no
**Priority**: 24

## Summary

`verify-harness.sh` は 13 チェックを実行するが、「docs が stale な状態」を検出して fail する仕組みがない。生成 docs の陳腐化を CI で検出したい。

## Current state

- `scripts/verify-harness.sh`: docs consistency check (生成 docs 一致) はある
- bootstrap 状態 / capability 状態の stale は検出しない
- pre-push hook で 17 チェック走るが、docs freshness は含まれない

## Acceptance

- [ ] `verify-harness.sh` に `--docs-fresh` チェックが追加される
- [ ] project-state.toml の `updated` 日付と current-state.md の実態が整合しない場合 fail
- [ ] pre-push hook で docs freshness が検証される

## References

- `scripts/verify-harness.sh`
- `docs/data/project-state.toml`
