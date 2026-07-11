# 実装計画の書き方

ステータス: 運用メモ（決定記録ではない）  
日付: 2026-07-11

---

## 目的

`docs/plans/` は、ADR / RFC で決まった方針を**どう実装するか**の計画を置く場所である。

| 置き場 | 役割 |
|--------|------|
| `docs/adr/` | 判断そのもの。進捗ダッシュボードにしない。 |
| `docs/rfcs/` | 詳細な設計・仕様。 |
| `docs/plans/` | Phase / PR レーン / 完了条件 / 検証コマンド。 |
| `issues/` | チケット単位の追跡（計画の実行面）。 |
| `docs/current-state.md` | いま動いている事実。 |
| `docs/design/` | 既存の設計メモ（例: `gc-implementation-plan.md`）。新規の実装計画は本ディレクトリを優先。 |

## ファイル命名

`NNN-kebab-slug.md`（例: `001-wasm-gc-phases.md`）。番号は計画専用で ADR / RFC と独立。

## 推奨構成

1. 対応 ADR / RFC / issue
2. 現状とゴール
3. フェーズ分割と完了条件
4. PR / 作業レーン（並列可否）
5. 検証コマンド
6. リスクと依存
7. 進捗の更新規則（誰が・いつ `current-state` / issue を更新するか）

進捗の生きた数字は計画本文へ継ぎ足し続けず、issue または `docs/current-state.md` へ寄せる。
