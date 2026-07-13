# ADR-047: コード品質ツールの分業と品質ゲート

ステータス: **PROPOSED** — `fmt` / `lint` / 人レビューの分業とゲート階層を固定する

提案日: 2026-07-13

調査正本: [`docs/research/code-quality-practices.md`](../research/code-quality-practices.md)

本提案は [ADR-001](ADR-001-harness-bootstrap.md) が「後続で扱う」とした
フォーマッタと lint スタック / CI 構造への応答である。ADR-001 の harness 全体決定は
有効のままとし、本 ADR はそれを SUPERSEDE しない。

---

## 文脈

Arukellt には既に次がある。

- selfhost の `arukellt fmt`（書式の正本は [`docs/language/formatter.md`](../language/formatter.md)）
- `arukellt lint` と警告コード台帳
- pre-commit での staged `.ark` に対する `fmt --check`
- `verify quick` 内の code-quality ratchet（[`docs/data/ark-code-quality-baseline.toml`](../data/ark-code-quality-baseline.toml)）
- 実装品質の文面規約（[`AGENTS.md`](../../AGENTS.md)）と設計寄りの規約
  （[`docs/process/coding-conventions.md`](../process/coding-conventions.md)）

欠けているのは、これらを**どの層が何を強制し、人は何に集中するか**という長期判断である。
外部調査（Go の `gofmt`、Prettier、Google のコードレビュー標準など）は、書式論争を
機械に委ね、レビューを設計に寄せる分業が大規模開発で安定すると示す。

命名・インデント幅・コメント文面などの詳細規約は本 ADR の対象外である。
正本は `AGENTS.md` / `coding-conventions.md` のままとする。

---

## 提案する決定

### 1. 役割分業

| 層 | 担当 | 典型例 |
|----|------|--------|
| `arukellt fmt` | 機械的な書式の**唯一基準** | インデント、空白、import 整列、改行レイアウト |
| `arukellt lint` および静的チェック | バグ・規約違反・危険パターンの検出 | unused、deprecated 利用、policy 監査 |
| 人間 / エージェントのレビュー | overall design、依存追加、テスト戦略、副作用、境界条件 | 抽象化の妥当性、層違反、契約変更 |

formatter で直せる指摘をレビューの主戦場にしない。linter で機械検出できる論点を
「好み」として議論しない。

### 2. ゲート階層

次の順で強化し、上位層は下位層の再検証または拡張とする。

1. ローカルでの `arukellt fmt`（保存時またはコミット前の自動修正）
2. pre-commit: staged `.ark` の `fmt --check`、続けて `verify quick`
3. CI での同等ゲート再実行
4. required status checks / branch 保護による未通過 merge の拒否（運用設定は
   repository 側。本 ADR は「機械ゲートを破って merge しない」方針を固定する）

機械で直せる・検出できる失敗を、レビューコメントだけで「お願い」しない。

### 3. 規約の置き場

- 読みやすさ・命名・分割・コメント・レビュー問い → [`AGENTS.md`](../../AGENTS.md)
- コンパイラ層・公開 API 形・エラー・決定性 → [`docs/process/coding-conventions.md`](../process/coding-conventions.md)
- ユーザー到達パスの panic 禁止 → [ADR-015](ADR-015-no-panic-in-user-paths.md)
- 本 ADR → **何を機械化し、何を人に残すか**、およびゲート階層のみ

### 4. 段階強化

- 新規違反から fail 化する。既存負債は baseline / ratchet で削減する。
- 既存コードベースを一斉に赤くして導入を失敗させることを既定にしない。
- ratchet 天井の緩和で回帰を隠さない。天井変更には根拠と追跡 issue を要する。

---

## 代替案（却下）

1. **レビューだけで品質を保つ** — 差分ノイズと style 論争が再発し、設計議論の時間が減る。
2. **書式ルールを lint に寄せる** — formatter とルールが競合し、自動修正の単一正本が消える。
3. **外部汎用フォーマッタを `.ark` に導入する** — 言語構文・コメント・import 規則の正本が
   selfhost から離れ、LSP / playground / CLI の一貫性が崩れる。

---

## 帰結（採択時）

- コントリビュータとエージェントは「まず fmt、次に lint / verify、設計はレビュー」の順を共有する。
- ADR-001 の未決メモ「フォーマッタと lint スタック」は本決定で閉じる。
- CODEOWNERS や複雑度ゲートの必須化は本 ADR に含めない
  （[`docs/research/code-quality-practices.md`](../research/code-quality-practices.md) の将来候補）。

## 再検討条件

- selfhost formatter が言語進化に追随できず、外部ツール分割が不可避になったとき。
- lint と fmt の境界が運用上衝突し、統合 CLI の方が総コストが低いと実証されたとき。
- マルチリポジトリ化や領域オーナーシップ必須化で、ゲート階層に ownership 層を足す必要が出たとき。

## 関連

- [`docs/research/code-quality-practices.md`](../research/code-quality-practices.md)
- [ADR-001](ADR-001-harness-bootstrap.md)
- [ADR-015](ADR-015-no-panic-in-user-paths.md)
- [ADR-048](ADR-048-design-heuristics-application-order.md)（提案）
- [`docs/language/formatter.md`](../language/formatter.md)
- [`docs/contributing.md`](../contributing.md)
