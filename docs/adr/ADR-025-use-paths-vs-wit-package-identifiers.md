# ADR-025: ソースモジュールパスと WIT パッケージ識別子 — 衝突ポリシーと構文探索

ステータス: **SUPERSEDED** — [ADR-031](ADR-031-import-syntax-wit-unification.md) に統合（探索メモ）
日付: 2026-04-16
トラック: language-design（issue #123）
後継: [ADR-031-import-syntax-wit-unification.md](ADR-031-import-syntax-wit-unification.md)
決定日: 2026-04-16

## 背景

Arukellt には、新規参加者やツールから見てどちらも「import」に見える、異なる 2 つの命名表面がある:

| 表面 | 例 | 役割 |
|---------|---------|------|
| Layer S — ソースモジュール | `use std::host::stdio` | コンパイル時に `.ark` モジュールと stdlib を解決（`::` パス）。 |
| Layer C — component / WIT | `wasi:cli/stdin@0.2.10` | Component Model バイナリ境界でパッケージとインターフェースを識別。 |

ADR-009 はこれらを**別レイヤー**として、異なる構文とキーワード（`use` と Layer C 向けに予約された `import`）で維持するよう**決定**した。本 ADR はその分離を**再オープンしない**。issue #123 向けに、ADR-009 を超える単一の詳細化アーティファクトとして、**追加の設計候補**、**衝突回避戦術**、将来の Layer C ソース形式の**非拘束の構文スケッチ**、**移行メモ**を記録する。

## 決定候補（要約）

### Candidate A — 単一形式を everywhere（ソースでも `namespace:pkg/interface@ver`）

- **考え方**: ソースの `::` パスを WIT 風パッケージ ID に置き換える（例: `use arukellt:std/io`）。
- **利点**: 単一のメンタルモデル。出力 component メタデータとの密接な整合。
- **欠点**: 大規模な破壊的変更。WIT ID は通常の言語モジュールに不向きなレジストリ/組織セマンティクスをエンコード。セルフホスティングと日常コードの ergonomics が悪化（ADR-009「Alternatives Considered A」参照）。
- **本ドラフトでの判定**: デフォルトとして **Rejected**。明示的な非目標のアンカーとしてのみ残す。

### Candidate B — 二層、異なる構文（ADR-009 デフォルト）

- **考え方**: Layer S は `use` + `::` パス。Layer C は WIT 文字列 / 専用形式で、`::` 解決規則を再利用しない。
- **利点**: パス文法と WIT 文法の衝突ゼロ。業界の一般的パターン（ソース import vs 外部 WIT ツール）に合う。既存フィクスチャと stdlib レイアウトを維持。
- **欠点**: 教える概念が 2 つ。ドキュメントを明示的にする必要（本 ADR + `docs/spec/import-system.md`）。
- **本ドラフトでの判定**: **Recommended default** — ADR-009 と一致。

### Candidate C — `wit` キーワードまたは属性ブリッジ

- **考え方**: `wit import "wasi:cli/stdin@0.2.10"` またはモジュール/アイテムへの `#[wit_import("…")]`。
- **利点**: 視覚的に明確。ビルドマニフェストのみに WIT を載せる場合は任意。
- **欠点**: 表面積が増える。ADR-009 は複合キーワードの代わりに Layer C 向けに素の `import` を予約した（そこで一部検討）。
- **判定**: 文字列リテラル `import` より強い曖昧さ解消が欲しい実装者向けの**任意バリアント**。二層デフォルトには不要。

### Candidate D — ソース外のみ WIT

- **考え方**: `.ark` に WIT テキストなし。world/interface は `.wit` と CLI フラグ（例: `--wit`）にあり、バインディングは生成または暗黙。
- **利点**: パーサ複雑さ最小。「WIT はツールチェーン入力」ワークフローに合う。
- **欠点**: 「単一ファイルで component 定義」例は不便。Layer S への生成シンボル可視性の話は依然必要。
- **判定**: 初期フェーズの**有効な配信経路**。Candidate B（Layer S 不変）と両立。

## 名前空間衝突の回避

1. **字句的形状**
   - Layer S パスは **`::`** と識別子セグメントを使う。WIT の **`ns:pkg`** コロン対や、WIT と同じトークン列パターンのパッケージ/インターフェース間 **`/`** は使わない。
   - WIT パッケージ ID は **`:`**（名前空間区切り）、**`/`**（インターフェース）、**`@`**（バージョン）、WIT ファイルでは任意で **`.{…}`**（シンボル列挙） — Arukellt 式構文ではない。

2. **キーワード分離**
   - **`use`** — 規範ドキュメントでは Layer S のみ。
   - **`import`** — レガシーファイル import（将来退役予定）。ADR-009 に従い退役後は Layer C 用に予約。

3. **計画された Layer C ソース形式**
   - 完全な WIT パッケージ/インターフェース文字列を載せる**文字列リテラル**を優先（下記スケッチ）し、字句解析器が `wasi:cli` を識別子のパスと解釈しないようにする。
   - 専用文法が規定されるまで、引用なしの `import wasi:cli/...` は避ける。非引用形式はパス、ジェネリクス、将来の演算子とのパーサ曖昧さを招く。

4. **概念的衝突（字句的ではない）**
   - **`std::io` は WIT パッケージ ID ではない**。明示的ブリッジ（stdlib host facade vs 生 WASI import）なしに `wasi:io/…` と interchangeable と文書化してはならない。
   - ツールはレイヤー間を**黙って**マップしてはならない。マッピングはコンパイラ / マニフェスト / バインディング生成器の明示設定に属する。

5. **WIT のみで組織名前空間を予約**
   - WIT エコシステム慣行に従う: 組織所有の名前空間（`wasi:`、ベンダー固有プレフィックス）。Arukellt 言語モジュールは `std::`、パッケージルートなど通常のパスのまま。

## 非拘束の構文スケッチ（ソース内 Layer C）

> **非規範。** 例示のみ。パーサ、キーワード配置、属性形式は将来の ADR/issue 決定の対象。

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
// String form: WIT package + interface + optional version inside quotes.
import "wasi:cli/stdin@0.2.10"

// Possible future: named binding for generated surface (spelling TBD).
// import "wasi:cli/stdin@0.2.10" as cli_stdin

// Layer S unchanged: ordinary module import.
use std::host::stdio
```

Layer C から生成されたバインディングは、Layer S では通常の import モジュール/型として現れる（正確な名前解決規則は実装 issue で TBD）。

## 移行と互換性

| フェーズ | Layer S | Layer C / `import` キーワード |
|-------|---------|-----------------------------|
| 現行 | `use` + レガシー `import foo`（兄弟モジュール） | WIT ID は `.wit` / ツールに現れる |
| 次の移行 | `import <single_identifier>` を廃止し `use` に（ADR-009） | `import` を Layer C 宣言に再利用 |
| 最終 | 既存フィクスチャは `::` パスを維持 | 新構文は additive |

**互換性の原則**

- `use std::…` の WIT 文字列への自動書き換えはしない。
- ドキュメント（`docs/spec/import-system.md`）で Layer S と Layer C を教え、`std::io` と `wasi:io/…` の LLM/ユーザー混同を防ぐ。

## 推奨デフォルト（本ドラフト）

**Candidate B** を継続デフォルトとして採用: Arukellt ソースパスと WIT パッケージ識別子文法を**統一しない**。厳格なレイヤー分離と上記衝突回避戦術を維持する。Layer C 表面は（Candidate C/D）本デフォルトを変えずに進化しうる。

## 他 ADR との関係

- **ADR-009**: 規範的 **ACCEPTED** 分離（WIT 向け `use` vs 予約 `import`）。本 ADR は issue #123 向けの**歴史的詳細化**。重複部分は ADR-031 に置き換えられる。
- **ADR-006**: ABI レイヤー — ソースセマンティクスと WIT ABI は別のまま。
- **ADR-023**: レジストリ解決は Layer S 依存に適用。モジュールパスを WIT ID に書き換えるものではない。

## 関連

- [ADR-009-import-syntax.md](ADR-009-import-syntax.md)
- [../spec/import-system.md](../spec/import-system.md)
- Issue #123 — import syntax and WIT package identifier unification policy
- Issue #124 — WIT component import / `--wit` wiring
