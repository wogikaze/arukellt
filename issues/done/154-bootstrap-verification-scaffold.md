# 横断基盤: `scripts/verify-bootstrap.sh` と fixpoint 検証 scaffold

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 154
**Depends on**: 153
**Track**: cross-cutting
**Blocks v1 exit**: no

## Summary

`docs/process/roadmap-cross-cutting.md` §6.5 は v5 で `scripts/verify-bootstrap.sh` による
Stage0 → Stage1 → Stage2 → fixpoint 検証を要求している。
まだセルフホスト本体は先だが、verify 導線と artifact 契約は先に scaffold 化しておいた方が、
v5 に入ったときの曖昧さを減らせる。

## 受け入れ条件

1. `scripts/verify-bootstrap.sh` の雛形が追加され、将来の Stage0/1/2/fixpoint 手順を受け止められる
2. 期待する artifact 命名、比較対象、失敗時の diff 出力方針が明文化される
3. `docs/compiler/bootstrap.md` の着手前提となる scaffold ができる
4. verify-harness から将来的に組み込み可能な契約が定義される

## 実装タスク

1. v5 roadmap と cross-cutting の bootstrap requirements を抽出する
2. script 雛形と usage / TODO セクションを作る
3. Stage artifact 名、比較方法、決定性前提を決める
4. process docs に bootstrap verify の入口を追加する

## 参照

- `docs/process/roadmap-cross-cutting.md` §6.5
- `docs/process/roadmap-v5.md`
- `scripts/verify-harness.sh`
