# Source of truth と継続検証の整合

**Status**: open
**Created**: 2026-03-31
**ID**: 285
**Depends on**: —
**Track**: main
**Blocks v1 exit**: no
**Priority**: 5

## Summary

実装量も issue 完了数も多いが、その分だけ `README`、`current-state`、個別 docs、scripts の間でズレが起き始めている。セルフホスト周りの記述差分はその典型。機能完成の定義を「コードがある」から「単一の source of truth から検証結果が自動生成される」に変える。未検証の scaffold、blocked な外部依存、暫定 docs、歴史的 docs を同じ棚に置かない。

## Current state

- `docs/current-state.md` の bootstrap 節が stale（fixpoint 達成済みだが「blocked」表記）
- `docs/compiler/bootstrap.md` と `scripts/verify-bootstrap.sh` に記述差分あり
- `README.md` と `current-state.md` の機能リストに差分がある可能性
- `scripts/generate-docs.py` が生成する docs と手書き docs の境界が不明確
- `issues/done/` の完了 issue と `current-state.md` の状態記述に齟齬が生じうる
- 歴史的 docs（`docs/migration/`, archived ADRs）と現在の docs が混在

## Acceptance

- [ ] `docs/current-state.md` が実態と完全に一致する（特に bootstrap 節の fixpoint 反映）
- [ ] `README.md` の機能リストが `current-state.md` と整合する
- [ ] `scripts/check-docs-consistency.py` が bootstrap 状態・capability 状態・component 状態も検証する
- [ ] 生成 docs と手書き docs の境界が明文化される（どのファイルが `generate-docs.py` 管轄か）
- [ ] 歴史的 / archived docs にその旨のバナーが付く
- [ ] `verify-harness.sh` が docs consistency check を含む（既存だが範囲を拡張）
- [ ] CI が「docs が stale な状態」を検出して fail する
- [ ] `issues/blocked/` の blocked 理由が upstream の現在の状態と一致する

## Approach

1. `docs/current-state.md` を全面レビューし、実態と合わせる（特に bootstrap、capability、component）
2. `README.md` と `current-state.md` の差分を特定し解消
3. `scripts/check-docs-consistency.py` の検証範囲を拡張
4. 生成 docs の一覧を `scripts/generate-docs.py` の冒頭コメントに明記
5. 歴史的 docs に `> **Historical**: ...` バナーを追加
6. `verify-harness.sh` に docs-freshness check を追加
7. `issues/blocked/037` の upstream 状態を確認し更新

## References

- `docs/current-state.md`
- `docs/compiler/bootstrap.md`
- `README.md`
- `scripts/check-docs-consistency.py`
- `scripts/generate-docs.py`
- `scripts/verify-harness.sh`
- `issues/blocked/037-jco-gc-support.md`
