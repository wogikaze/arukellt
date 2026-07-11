# ADR-014: 言語仕様と Stdlib API の安定性ラベル

ステータス: **ACCEPTED** — 言語仕様ラベルと stdlib API lifecycle ラベルを採用
日付: 2026-04-01  
決定者: core team
決定日: 2026-04-01

## 文脈

Arukellt の言語仕様と stdlib は拡大している。ユーザーは、本番コードで頼ってよい機能と
変更されうる機能を知る必要がある。明示的な安定性保証がなければ、採用リスクを判断できない。

## 決定

言語機能と公開 stdlib API エントリのすべてに、次の 4 段階分類を適用する。

| ラベル | 定義 |
|--------|------|
| `stable` | 仕様を凍結。破壊的変更は事前告知付きの移行ガイド経由のみ。 |
| `provisional` | 概ね正しいが細部は変わりうる。告知なしの破壊的変更はしない。 |
| `experimental` | 設計作業中。事前告知なしの破壊的変更がありうる。 |
| `unimplemented` | 仕様はあるが未実装。使用するとコンパイルエラー。 |

### 言語仕様への適用

- `docs/language/spec.md` の各節見出しの直後に `<!-- stability: LABEL -->` コメントを付ける。
- 節レベルのラベルはその節の多数を表し、個別項目にはインライン注釈を付けてよい。

### stdlib への適用

- `std/manifest.toml` の各公開エントリに `stability` フィールドを持つ。
- 許可値: `stable`, `provisional`, `experimental`, `deprecated`。`deprecated` は callable な移行状態であり、`deprecated_by` を必須とする。
- `unimplemented` は言語仕様の状態であり、公開 stdlib manifest entry の値には使わない。
- `scripts/gen/generate-docs.py` が生成する stdlib リファレンスは、関数表に安定性ラベルを表示する。

## 帰結

- ユーザーは stdlib リファレンスを安定性レベルでフィルタできる。
- コンパイラは将来、`experimental` 使用時に警告を出せる（別途追跡）。
- `provisional` から `stable` へ昇格するときは changelog エントリが必須。
- 降格・削除するときは移行ガイドエントリが必須。

## 参照

- `docs/language/spec.md`
- `std/manifest.toml`
- `scripts/gen/generate-docs.py` — `format_stability_counts()` とリファレンス表の安定性列
- ADR-013: プライマリターゲットの tier（ターゲット層の関連する安定性概念）
