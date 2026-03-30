# README を現在の利用可能面を正確に示す入口に更新する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 222
**Depends on**: 221
**Track**: main
**Blocks v1 exit**: yes

## Summary

README が宣伝文・設計理想・現行実装を混在させており、初見の利用者が「今使えるもの」を正しく把握できない。
この issue では README を「現在の利用可能面を正確に示す入口」として再編する。
詳細は `docs/current-state.md` に委ねる構成とする。

## Acceptance

- [ ] README の冒頭に「今何ができるか」が 5 行以内で明確に書かれている
- [ ] 「実験的」「未実装」「設計のみ」な機能が README 上で誤解なく識別できる
- [ ] Quickstart セクションが実際に動くコマンドのみで構成されている
- [ ] 詳細状態は `docs/current-state.md` へのリンクで案内している

## Scope

### README 構成の見直し

- 現在の README から設計理想・将来構想の記述を分離
- "What works today" セクションの追加
- インストール・Quickstart が実際に動く手順であることを確認

### 実験機能・未実装機能の表記

- 実験機能には `[experimental]` 表記を追加
- 未実装機能は README から削除するか明示的にラベル付け
- 将来構想は `docs/roadmap.md` 等に移動

### リンク構造の整理

- `docs/current-state.md` への参照を起点にした情報アーキテクチャの整備
- 各機能の詳細ページへのナビゲーション

## References

- `README.md`
- `docs/current-state.md`
- `issues/open/221-rebuild-current-state-as-single-source.md`
