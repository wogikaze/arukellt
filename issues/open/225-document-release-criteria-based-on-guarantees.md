# リリース可否基準を「何を保証するか」ベースで文書化する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 225
**Depends on**: 221, 223
**Track**: main
**Blocks v1 exit**: yes

## Summary

現状、リリース判断の基準が「何ができるか」になっており、利用者への保証が不明確である。
「動く機能がある」ことと「その動作を保証する」ことは異なる。
この issue では、v1 リリース可否を判断するための「保証ベースの基準」を文書化する。

## Acceptance

- [ ] v1 リリースに必要な保証事項のリストが文書化されている
- [ ] 各保証事項に対応する検証方法（テスト・CI・手動確認など）が明記されている
- [ ] 「保証しない（experimental）」面の明示的な列挙がある
- [ ] リリースチェックリストが存在し、実行可能である

## Scope

### 保証範囲の設計

- v1 で保証する言語機能・API・動作の列挙
- 保証しない面（experimental・未実装）の明示的な境界定義
- 保証レベルの定義（クラッシュしない / 仕様通りに動く / 将来も壊れない など）

### 検証方法の文書化

- 各保証事項に対するテスト戦略の記述
- CI ゲートとの対応関係
- 手動確認が必要な項目のリスト

### リリースチェックリスト

- v1 リリース前に確認するチェックリストの作成
- `docs/release-criteria.md` または ADR として保存

## References

- `docs/current-state.md`
- `docs/adr/`
- `issues/open/221-rebuild-current-state-as-single-source.md`
- `issues/open/241-define-primary-target-and-tier-others.md`
