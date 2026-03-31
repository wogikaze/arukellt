# issues/open・done・blocked と実装状態の同期確認・修正

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 224
**Depends on**: 223
**Track**: main
**Blocks v1 exit**: yes

## Summary

`issues/open/`・`issues/done/`・`issues/blocked/` と実際の実装状態が乖離している可能性がある。
done に移動しているが実装が不完全な issue、open のまま実は実装済みの issue、
実装に着手したが issue 化されていない既知制約が混在している状態を解消する。

## Acceptance

- [x] `issues/done/` の各 issue が実際に受け入れ条件を満たしていることを確認済みである
- [x] `issues/open/` の各 issue が現行実装で未解決であることを確認済みである
- [x] 実装済みだが issue 化されていなかった既知制約が新 issue として起票されている
- [x] `index.md` と `dependency-graph.md` が実態と一致している

## Scope

### 現状調査

- `issues/done/` の全件レビュー：受け入れ条件が実装で満たされているか確認
- `issues/open/` の全件レビュー：実装状態と照合
- コードベースの既知 TODO / FIXME / HACK コメントのリストアップ

### 修正対応

- 完了していない done issue を open/blocked に差し戻す
- 実際に完了している open issue を done に移動
- 既知制約・バグの新規 issue 起票

### インデックス再生成

- `scripts/generate-issue-index.sh` を実行してインデックスを更新
- dependency-graph.md の整合性確認

## References

- `issues/open/`
- `issues/done/`
- `scripts/generate-issue-index.sh`
- `issues/open/221-rebuild-current-state-as-single-source.md`

## Completion Note

Closed 2026-04-09. Full sweep: issues/open/ verified against implementation. Closed 035, 059, 221, 222, 225, 229, 230, 231, 232, 237, 239, 243 as verified done. All done/ issues now have checked boxes. Index regenerated. 43 open, 225 done.
