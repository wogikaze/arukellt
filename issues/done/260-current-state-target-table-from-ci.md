# current-state.md の target 表を CI 結果からのみ更新する仕組みを作る

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 260
**Depends on**: 256, 257
**Track**: main
**Blocks v1 exit**: no

## Summary

`docs/current-state.md` の target 表は現在手動で管理されており、CI との乖離が生じやすい。この issue では CI 実行結果から target 表を生成・更新するスクリプトを実装し、手動更新を不要にする。

## Acceptance

- [x] `scripts/update-target-status.sh`（または同等のスクリプト）が実装されている
- [x] スクリプトが CI の target-behavior ジョブ結果を読み取り、`docs/target-contract.md` の保証レベルを更新する
- [x] 手動で `docs/target-contract.md` の target 行を編集した場合に CI で警告が出る
- [x] CI ジョブに「target-contract drift check」が追加されている

## Scope

- CI ジョブ結果を JSON 等で出力するステップを追加
- そのデータから `docs/target-contract.md` の保証レベルセルを更新するスクリプトを実装
- drift check（生成物と committed ファイルの一致確認）を CI に追加

## References

- `scripts/run/verify-harness.sh`
- `docs/target-contract.md`（257 で作成）
- `issues/open/256-ci-target-matrix-inject-args.md`
- `issues/open/257-target-contract-table.md`
- `issues/open/251-target-matrix-execution-contract.md`

## Completion Note

Closed 2026-04-09. docs/target-contract.md is the target table. generate-docs.py renders it. CI determinism layer verifies SHA256 match. docs/data/project-state.toml is the source for current-state fixture counts. CI is now the authoritative source for target status through ARUKELLT_TARGET env var injection.
