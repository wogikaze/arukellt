# ADR-047: コード品質ツールの分業と品質ゲート

ステータス: **ACCEPTED** — `fmt` / `lint` / 人レビューの分業とゲート階層を固定する

提案日: 2026-07-13
採択日: 2026-07-13

調査正本: [`docs/research/code-quality-practices.md`](../research/code-quality-practices.md)
実行正本: [`docs/data/code-quality-rules.toml`](../data/code-quality-rules.toml)

本決定は [ADR-001](ADR-001-harness-bootstrap.md) が「後続で扱う」とした
フォーマッタと lint スタック / CI 構造への応答である。ADR-001 の harness 全体決定は
有効のままとし、本 ADR はそれを SUPERSEDE しない。

---

## 文脈

Arukellt には既に次がある。

- selfhost の `arukellt fmt`（書式の正本は [`docs/language/formatter.md`](../language/formatter.md)）
- `arukellt lint` と警告コード台帳
- pre-commit での staged `.ark` に対する `fmt --check` と lint 二層
- `verify quick` 内の code-quality ratchet（[`docs/data/ark-code-quality-baseline.toml`](../data/ark-code-quality-baseline.toml)）
- 実装品質の文面規約（[`AGENTS.md`](../../AGENTS.md)）と設計寄りの規約
  （[`docs/process/coding-conventions.md`](../process/coding-conventions.md)）

本 ADR は、これらを**どの層が何を強制し、人は何に集中するか**、および運用契約
（severity・baseline・suppression・bypass・CI 階層）を再解釈不要な粒度で固定する。

命名・インデント幅・コメント文面などの詳細規約は本 ADR の対象外である。
正本は `AGENTS.md` / `coding-conventions.md` / rule registry のままとする。

---

## 決定

### 1. 役割分業（5 層）

| 層 | 担当 | 対象 | 判定 |
|----|------|------|------|
| 書式 | formatter | 空白、改行、インデント、括弧、import 整形 | 常に自動修正・gate fail |
| 静的品質 | linter / analyzer | 未使用、危険処理、曖昧構文、禁止パターン | 原則 gate fail（下記 severity） |
| 構造 | repository check | 依存方向、SSOT、生成物、公開面、巨大化 ratchet | fail または ratchet |
| 設計 | reviewer | 名前、抽象化、責務、API、変更容易性 | 人間・エージェントレビュー |
| 傾向 | metrics | 複雑度、長さ、重複、churn、coupling | hotspot 抽出。単独では fail しない |

同じ論点を複数層で判定しない。行幅・空白は formatter、禁止パターンは linter、
設計判断はレビューのみ。

### 2. 実施契約（再解釈禁止）

| 論点 | 固定内容 |
|------|----------|
| formatter | ファイル種別ごとに formatter は 0 または 1。正本は [`docs/data/tooling-inventory.toml`](../data/tooling-inventory.toml)。`.ark` は selfhost `arukellt fmt` のみ |
| linter | formatter と競合する style rule を持たせない。書式指摘は lint に載せない |
| severity | **error**: ゲート非 0。**warning**: 印刷するが通常 exit 0（`--deny` で error 昇格可）。**advisory**: レポート / レビュー用。単独では fail しない |
| autofix | 自動修正可能な rule は手動修正を要求しない。`.ark` 書式は `fmt`、lint に autofix が無い項目は `fmt` へ委譲するか未対応を明示 |
| baseline | 既存違反は [`ark-code-quality-baseline.toml`](../data/ark-code-quality-baseline.toml) 等で件数固定。新規違反は禁止。天井緩和には根拠と追跡 issue ID |
| suppression | allow/deny / ignore には rule ID、具体的理由、狭い対象、issue または ADR、owner、削除条件、期限または再評価条件を要する。`disable all`・無理由 file ignore・CI の `\|\| true`・永久 baseline は禁止 |
| changed-code policy | staged / 変更ファイルは baseline より厳しく扱う。pre-commit は touched `.ark` に `fmt --check` と `--deny prefer-else-if`（package は `--local`） |
| CI 階層 | **local**（開発者）→ **quick**（`verify quick` / PR）→ **full**（全体）→ **release**（release 契約）。canonical 入口は `scripts/manager.py` |
| review 分業 | formatter / linter が処理できる項目を PR checklist から除外する。設計判断は [ADR-048](ADR-048-design-heuristics-application-order.md) の順序 |
| emergency bypass | `--no-verify` や required check 回避の後は、同一作業単位で issue または incident 記録を必須とする。無記録 bypass は禁止 |

### 3. ゲート階層（運用）

1. **local**: `python3 scripts/manager.py fmt` と任意の `lint`
2. **pre-commit**: staged `.ark` の `fmt --check` → lint 二層（`--local` または full）+ `--deny prefer-else-if` → `verify quick`
3. **CI quick**: 品質専用 job（`quality-format` / `quality-lint`）と既存 verification。ローカルと同じ実装を呼ぶ
4. **required checks**: 未通過 merge を拒否（repository ruleset。手順は [`docs/process/ci-required-checks.md`](../process/ci-required-checks.md)）

### 4. 規約の置き場

| 文書 | 役割 |
|------|------|
| 本 ADR | 分業・severity・baseline・CI・bypass の決定 |
| [`code-quality-rules.toml`](../data/code-quality-rules.toml) | 実行可能な rule SSOT |
| [`tooling-inventory.toml`](../data/tooling-inventory.toml) | 拡張子ごとの canonical formatter/linter |
| [`AGENTS.md`](../../AGENTS.md) | 短い利用者向け規則 |
| [`docs/process/pr-review-checklist.md`](../process/pr-review-checklist.md) | 設計レビュー順序 |
| research 文書 | 背景。決定の正本にしない |

### 5. 段階強化

- Stage A（全面強制）: tabs/spaces 混在、trailing whitespace、final newline、formatter 差分、生成物 dirty、明確な未使用 import 等
- Stage B（baseline）: 長行、thin wrapper、複雑度信号等。件数固定 + 新規禁止 + touched ratchet
- Stage C（semantic cleanup）: 個別 issue。本 ADR の範囲外の実行計画

---

## 代替案（却下）

1. **レビューだけで品質を保つ** — 差分ノイズと style 論争が再発し、設計議論の時間が減る。
2. **書式ルールを lint に寄せる** — formatter とルールが競合し、自動修正の単一正本が消える。
3. **外部汎用フォーマッタを `.ark` に導入する** — 言語構文・コメント・import 規則の正本が
   selfhost から離れ、LSP / playground / CLI の一貫性が崩れる。

---

## 帰結

- コントリビュータとエージェントは「まず fmt、次に lint / verify、設計はレビュー」の順を共有する。
- ADR-001 の未決メモ「フォーマッタと lint スタック」は本決定で閉じる。
- 複雑度の単独 fail 化は含めない（[ADR-048](ADR-048-design-heuristics-application-order.md) および metrics 方針）。

## 再検討条件

- selfhost formatter が言語進化に追随できず、外部ツール分割が不可避になったとき。
- lint と fmt の境界が運用上衝突し、統合 CLI の方が総コストが低いと実証されたとき。
- マルチリポジトリ化や領域オーナーシップ必須化で、ゲート階層に ownership 層を足す必要が出たとき。

## 関連

- [`docs/research/code-quality-practices.md`](../research/code-quality-practices.md)
- [ADR-001](ADR-001-harness-bootstrap.md)
- [ADR-015](ADR-015-no-panic-in-user-paths.md)
- [ADR-048](ADR-048-design-heuristics-application-order.md)
- [`docs/language/formatter.md`](../language/formatter.md)
- [`docs/contributing.md`](../contributing.md)
