---
Status: done
Created: 2026-03-28
Updated: 2026-03-30
ID: 248
Track: docs
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: no
---

# migration guide v3→v4: MIR optimization pipeline completed
元ドキュメント: `docs/migration/v3-to-v4.md`（issue 化により移動済み）
# migration guide v3→v4: MIR optimization pipeline completed

## Summary

v3→v4 の MIR 最適化パイプライン実装が完了したことの追跡 issue。
`--opt-level` フラグ、7 つの MIR 最適化パス（const_folding, dce 他）、`--time` フラグ、
`ARUKELLT_DUMP_PHASES` 環境変数は完了している。
v4 は完全な後方互換であり、ユーザーコードの変更は不要。

元ドキュメント: `docs/migration/v3-to-v4.md`（issue 化により移動済み）

## Acceptance

- [x] `--opt-level 0|1|2` フラグが機能する
- [x] `--opt-level 1` がデフォルトで `const_folding`, `dce` を適用する
- [x] `--opt-level 2` が全 7 パスを適用する
- [x] `--time` フラグがフェーズ別コンパイル時間を stderr に出力する
- [x] `ARUKELLT_DUMP_PHASES=optimized-mir` が MIR をダンプする
- [x] 既存の T1/T3 プログラムがすべて変更なくコンパイルされる

## User Migration Checklist

以下はユーザーコード側の対応事項（言語実装の完了条件ではない）：

- [x] ソース変更不要 — v4 は完全後方互換
- （任意）リリースビルドスクリプトに `--opt-level 2` を追加
- （任意）CI のコンパイル時間回帰を追跡するために `--time` を追加
- （任意）`scripts/update-baselines.sh` でベンチマーク baseline を更新
- （任意）`ARUKELLT_DUMP_PHASES=optimized-mir` で最適化効果を確認

## References

- `crates/ark-mir/src/opt/`
- `docs/current-state.md`