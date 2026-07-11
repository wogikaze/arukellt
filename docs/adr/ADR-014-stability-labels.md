# ADR-014: 言語仕様と Stdlib API の安定性ラベル

ステータス: **ACCEPTED** — 4段階の安定性ラベル（stable/provisional/experimental/unimplemented）を採用  
日付: 2026-04-01  
決定者: core team

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
- 許可値: `stable`, `provisional`, `experimental`。
- `scripts/gen/generate-docs.py` が生成する stdlib リファレンスは、関数表に安定性ラベルを表示する。

### 現行ベースライン（2026-04-01）

**言語仕様:**
- 節 1–6, 8, 10（字句・型・式・文・パターン・アイテム・演算子優先順位）: `stable`
- 節 7（モジュールシステム）: `provisional`
- 節 9（stdlib）: 各関数エントリを参照

**Stdlib manifest:**
- 公開関数 267 件すべてに明示的な stability フィールドがある。
- 多数は `provisional`。`stable` はまだない（v1 凍結待ち）。現時点で `experimental` もない。

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
