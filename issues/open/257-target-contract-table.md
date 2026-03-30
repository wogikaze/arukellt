# target ごとの検証面テーブルを定義する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 257
**Depends on**: 251
**Track**: main
**Blocks v1 exit**: yes

## Summary

各 target について「何が保証されているか」を表にした実行可能な target contract が存在しない。`docs/current-state.md` の target 表は保証面が分解されていないため、CI との対応が取れない。

## Acceptance

- [ ] `docs/target-contract.md` が作成されている
- [ ] target × 検証面（parse / check / compile / run / emit-core / emit-component / wit / host-capability / determinism / validator-pass）のマトリクスが定義されている
- [ ] 各セルに保証レベル（guaranteed / smoke / scaffold / none）が明記されている
- [ ] T1/T3（実装済み）と T2/T4/T5（未実装）がそれぞれ異なる保証レベルで記載されている

## Scope

- `docs/target-contract.md` の新規作成
- コードベースと照合して現実の保証範囲を確認
- `docs/current-state.md` から target 表の重複記述を削除して `docs/target-contract.md` へ参照を張る

## References

- `docs/current-state.md`
- `docs/data/project-state.toml`
- `issues/open/251-target-matrix-execution-contract.md`
