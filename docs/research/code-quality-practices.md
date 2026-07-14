# 良いコード実務の調査 — Arukellt への取り込み判定

ステータス: 調査メモ（決定記録ではない）  
日付: 2026-07-13  
関連提案: [ADR-047](../adr/ADR-047-code-quality-tooling-and-gates.md)、[ADR-048](../adr/ADR-048-design-heuristics-application-order.md)  
既存の関連決定: [ADR-001](../adr/ADR-001-harness-bootstrap.md)、[ADR-015](../adr/ADR-015-no-panic-in-user-paths.md)

---

本ファイルは、大規模開発における「良いコード」実務（formatter / linter 分業、
設計原則、品質ゲート、メトリクス、レビュー文化）の調査結果を、Arukellt の
現行資産に照らして評価した記録である。言語仕様の前提にはしない。
拘束する判断は ADR-047 / ADR-048（いずれも ACCEPTED）に置く。

情報源の優先順位は公式スタイルガイド・主要 OSS の CONTRIBUTING・学術メトリクス
文献・企業のコードレビュー標準である。5 言語（Java / JS·TS / Python / Go / C#）
横断のツール表そのものは arukellt に持ち込まない。

---

## 調査範囲から arukellt に効く論点

外部調査が示す中心命題は次のとおりである。

1. 良いコードの中心は美意識ではなく、**機械で強制できる一貫性**・**変更容易性**・
   **レビューしやすい差分**・**運用で崩れにくいガバナンス**である。
2. **formatter** は機械整形、**linter / analyzer** はバグ・規約違反・危険パターン、
   **レビュー** は設計妥当性、という分業が最も安定する。
3. 設計原則（SOLID / DRY / KISS / YAGNI）は張力を持つヒューリスティクスであり、
   **KISS / YAGNI を先に、SOLID / DRY を必要箇所へ後から**が破綻しにくい。
4. メトリクスは品質そのものではなく**危険信号**として使う。
5. 規約は文書で終わらせず、pre-commit → CI → required checks → レビュー基準へ
   写像して初めて「運用」になる。

---

## 現状マップ（Arukellt）

| 領域 | 現行の正本 / 仕組み | 備考 |
|------|---------------------|------|
| 書式 | `arukellt fmt`、[`docs/language/formatter.md`](../language/formatter.md) | selfhost formatter が唯一基準 |
| 静的警告 | `arukellt lint`、[`docs/data/warnings.toml`](../data/warnings.toml) | provisional だが functional |
| 実装品質規約 | [`AGENTS.md`](../../AGENTS.md)「コード品質規約」 | 命名・コメント・分割・レビュー問い |
| 設計寄りの規約 | [`docs/process/coding-conventions.md`](../process/coding-conventions.md) | API・層・エラー・決定性・テスト配置 |
| ユーザー経路品質 | [ADR-015](../adr/ADR-015-no-panic-in-user-paths.md) | panic / unwrap 禁止 |
| ローカルゲート | pre-commit: staged `.ark` の `fmt --check` → `lint --local`（package modules）/ full lint（standalone）→ `verify quick` | [`docs/contributing.md`](../contributing.md) |
| ratchet | `scripts/check/check-ark-code-quality.py` + [`docs/data/ark-code-quality-baseline.toml`](../data/ark-code-quality-baseline.toml) | タブ・極端インデント・長行など |
| harness | [ADR-001](../adr/ADR-001-harness-bootstrap.md) | 「formatter と lint スタック」は後続予定のまま |

既に揃っているものは多い。欠けているのは主に、**分業とゲート階層を長期判断として
固定すること**、および**設計原則の適用順序を明示すること**である。

---

## 採用 / 見送り判定

| 調査上の主張 | 判定 | arukellt での扱い |
|--------------|------|-------------------|
| formatter と linter の役割分離 | **採用** | ADR-047。`fmt` / `lint` / 人レビューの分業 |
| ゲート階層（local → pre-commit → CI → required checks） | **採用** | ADR-047。既存 hook / `verify quick` を公式層として文書化 |
| レビューは設計重視、style nit を主戦場にしない | **採用** | ADR-047 + 既存 AGENTS レビュー基準 |
| KISS / YAGNI → DRY / SOLID の適用順 | **採用** | ADR-048 |
| メトリクスは危険信号（単独合否にしない） | **採用（方針のみ）** | ADR-048。必須 CI fail 化はしない |
| 命名・コメント Why・モジュール責務 | **既採択相当** | AGENTS.md / coding-conventions。再 ADR 化しない |
| Java Checkstyle+PMD 等の言語別スタック表 | **見送り** | `.ark` は自前 fmt/lint。外部スタックを真似ない |
| Biome / Ruff 統合ツール論の丸写し | **見送り** | 既に CLI が分かれている。統合は必要が生じたとき再評価 |
| 複雑度しきい値の即時 fail 化 | **見送り** | 信号用途のみ。gate 化は別 plan / issue |
| CODEOWNERS 必須化 | **将来候補** | 単一リポジトリ・エージェント駆動の現状では時期尚早 |
| 公開 API doc comment の強制 | **将来候補** | `///` / `//!` は言語にある。強制範囲は issue 前提で段階導入 |

---

## ADR 化対象

| ADR | なぜ ADR か |
|-----|-------------|
| [ADR-047](../adr/ADR-047-code-quality-tooling-and-gates.md) | ADR-001 が後続とした formatter/lint/CI の分業を閉じる。代替（レビューだけ、lint に書式を寄せる、外部 formatter）があり、長期拘束が必要 |
| [ADR-048](../adr/ADR-048-design-heuristics-application-order.md) | SOLID 全面適用 vs 単純解優先など複数の妥当な方針があり、エージェント・人間の設計判断を揃える必要がある |

命名・インデント・コメント文面の詳細は ADR にしない。正本は `AGENTS.md` のままとする。

---

## 実装済みの追跡

`python3 scripts/manager.py quality report` は CQ-13 で file/function size、
parameter count、nesting、近似 complexity、compiler-local fan-in/fan-out、public
function、thin wrapper、長行、TODO/FIXME、lint suppression、git churn、dependency
centrality を収集する。分布は `docs/data/ark-code-quality-baseline.toml` に明示操作でのみ
保存し、hotspot は ADR-048 の正規化式で並べる。値自体を新しい hard gate にはしない。

## 将来候補（この調査では決定しない）

1. **CODEOWNERS / rulesets** — 領域責任者が増え、パス単位の承認がボトルネックより
   価値が高くなったとき。
2. **公開 API の doc comment 規約の段階強制** — stdlib / 公開モジュールから
   `///` 要約を必須化し、内部実装コメントは Why のみ（AGENTS と整合）。
3. **lint ルールの fail 範囲拡大** — 新規違反から fail、既存は baseline 削減
   （ADR-047 の段階強化方針に従う）。

実装・設定変更は別 issue / plan で追跡する。

---

## 関連

- [`AGENTS.md`](../../AGENTS.md) — コード品質規約
- [`docs/process/coding-conventions.md`](../process/coding-conventions.md)
- [`docs/language/formatter.md`](../language/formatter.md)
- [`docs/contributing.md`](../contributing.md)
- [ADR-001](../adr/ADR-001-harness-bootstrap.md)
- [ADR-015](../adr/ADR-015-no-panic-in-user-paths.md)
- [ADR-047](../adr/ADR-047-code-quality-tooling-and-gates.md)（ACCEPTED）
- [ADR-048](../adr/ADR-048-design-heuristics-application-order.md)（ACCEPTED）
