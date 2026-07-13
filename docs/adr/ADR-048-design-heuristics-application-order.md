# ADR-048: 設計原則の適用順序

ステータス: **ACCEPTED** — KISS / YAGNI を先に、DRY / SOLID を必要箇所へ後から適用する

提案日: 2026-07-13
採択日: 2026-07-13

調査正本: [`docs/research/code-quality-practices.md`](../research/code-quality-practices.md)

PR / エージェントは本 ADR の判断手順を
[`docs/process/pr-review-checklist.md`](../process/pr-review-checklist.md) および
[`AGENTS.md`](../../AGENTS.md) と**同じ順序**で用いる。順序の再解釈は禁止する。

---

## 文脈

SOLID・DRY・KISS・YAGNI・Kent Beck の simple design は、いずれも変更容易性を
目指すが、相互に張力を持つ。SOLID の過剰適用は抽象階層を増やし、DRY を
「コピペ禁止」と誤読すると局所性が壊れ、YAGNI を無視すると未使用の
extension point が保守コストになる。

Arukellt の [`AGENTS.md`](../../AGENTS.md) は既に次を求めている。

- 薄い転送 wrapper や将来のための placeholder を作らない
- 抽象化は重複だけでなく認知負荷・移動回数も減らすこと
- 関数・ファイルの行数を単独の品質指標にしない

本 ADR は、これらの実務規約の上位にある**原則の適用順序**を固定する。
個別の命名・コメント文面の詳細は対象外（コメント種別は AGENTS / coding-conventions）。

---

## 決定

### 1. 原則の扱い

SOLID / DRY / KISS / YAGNI および simple design は、暗記して全面適用する絶対規則ではなく、
**相互に張力を持つヒューリスティクス**として扱う。原則違反の指摘より、
変更容易性・可読性・テスト可能性の実測を優先する。

「SOLID に違反」「DRY ではない」という抽象的指摘は禁止する。必ず具体的な
変更圧力、同期漏れ、責務混在、依存問題を示す。

### 2. 判断手順（固定・9 ステップ）

設計判断は次の順でのみ行う。PR checklist とエージェント指示もこの順序に写す。

1. 現在必要な振る舞いと契約を特定する。
2. 最も直接的で単純な実装を選ぶ（KISS）。
3. 未確定の将来要求を実装していないか確認する（YAGNI）。
4. データと責務の owner が一意か確認する。
5. 重複が「同じ知識」か「偶然似ているコード」か区別する。
6. 同じ知識なら DRY を適用する。
7. 変更理由が異なる責務が混ざっている場合だけ SOLID（局所）を適用する。
8. 二つ目の実例がない extension point や interface は原則作らない。
9. コードで表せない制約と判断だけコメントまたは ADR に残す。

```text
現行の振る舞いと契約
  → 最も単純な実装？ —yes→ 採用
  → 将来要求の推測が入っていないか（YAGNI）
  → owner は一意か
  → 同じ知識の重複？ —yes→ DRY
  → 変更理由の異なる責務混在？ —yes→ 局所 SOLID
  → 二例目のない抽象を増やしていないか
  → コードで表せない理由だけをコメント/ADR へ
```

### 3. 抽象化の合格条件

新しい抽象・helper・ファイル分割は、次を満たすときに限る。

- 呼び出し側が責任を予測しやすくなる
- 理解のための往復（薄い facade、無意味なファイル分割）が増えない
- 重複削減だけでなく、認知負荷も減る（[`AGENTS.md`](../../AGENTS.md) と一致）

### 4. メトリクス

- 行数・ファイル数・循環的複雑度・認知的複雑度を、**単独の合否ゲートにしない**。
- これらは hotspot 抽出やレビュー優先度のための**危険信号**として使ってよい。
- 信号を CI fail や必須閾値にする場合は、本 ADR ではなく別の plan / issue で
  根拠・段階導入・誤検知対策を定める。
- 優先順位の目安は `complexity × churn × dependency centrality`（複雑でも静的な
  経路より、頻繁に変わる orchestration を先に直す）。

---

## 代替案（却下）

1. **SOLID を常時全面適用する** — 早期の interface / 層が増え、YAGNI と衝突する。
2. **DRY をコピペ禁止として厳格適用する** — 偶然似た形まで共通化し、変更が結合する。
3. **複雑度しきい値を直ちに CI fail にする** — 既存コードと正当な分岐密度の多い
   経路を一斉に罰し、導入が失敗しやすい。

---

## 帰結

- 設計レビューとエージェント実装は「まず単純解、知識重複が育ってから抽象化」を既定にする。
- [`docs/process/coding-conventions.md`](../process/coding-conventions.md) の層所有や
  API 形とは直交する。本 ADR は「いつ抽象化するか」の順序である。
- 品質ツール分業は [ADR-047](ADR-047-code-quality-tooling-and-gates.md) が担当する。

## 再検討条件

- マルチパッケージ / 公開ライブラリ境界が増え、DIP 等を初期から強制した方が
  壊れにくいと実証されたとき。
- 複雑度信号の運用データが揃い、限定パスでの閾値 fail が誤検知より価値が高いと
  示されたとき（別 ADR または plan で閾値を提案する）。

## 関連

- [`docs/research/code-quality-practices.md`](../research/code-quality-practices.md)
- [ADR-047](ADR-047-code-quality-tooling-and-gates.md)
- [`AGENTS.md`](../../AGENTS.md)
- [`docs/process/coding-conventions.md`](../process/coding-conventions.md)
- [`docs/process/pr-review-checklist.md`](../process/pr-review-checklist.md)
