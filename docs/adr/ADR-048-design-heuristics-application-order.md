# ADR-048: 設計原則の適用順序

ステータス: **PROPOSED** — KISS / YAGNI を先に、DRY / SOLID を必要箇所へ後から適用する

提案日: 2026-07-13

調査正本: [`docs/research/code-quality-practices.md`](../research/code-quality-practices.md)

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

本 ADR は、これらの実務規約の上位にある**原則の適用順序**を固定し、
エージェントと人間の設計判断を揃える。個別の命名・コメント文面は対象外である。

---

## 提案する決定

### 1. 原則の扱い

SOLID / DRY / KISS / YAGNI および simple design は、暗記して全面適用する絶対規則ではなく、
**相互に張力を持つヒューリスティクス**として扱う。原則違反の指摘より、
変更容易性・可読性・テスト可能性の実測を優先する。

### 2. 適用順序

現行要件に対して次の順で判断する。

1. **KISS** — 今の問題を最も単純な実装で解けるか。解けるならそれを選ぶ。
2. **YAGNI** — 未確定の将来要件のための機能・設定・extension point を今入れない。
3. **責務分割** — 単純実装では解けない、または理解・変更が破綻するときだけ分割する。
4. **DRY** — 似た形のコードがあるだけでは足りない。**同一の業務知識・仕様知識が
   複数箇所で別表現され、更新同期が必要になる**ときに単一の権威的表現へ寄せる。
5. **SOLID（局所）** — レイヤ境界や依存方向（例: domain が infrastructure に依存しない）が
   実際に必要になった箇所だけ適用する。interface 増殖を目的化しない。

```text
現行要件
  → 単純実装で解ける？ —yes→ KISS を維持
  → no → 責務分割（必要なら局所 SOLID）
  → 知識重複がある？ —yes→ DRY
  → no → YAGNI を維持
  → テストで振る舞いを固定
```

### 3. 抽象化の合格条件

新しい抽象・helper・ファイル分割は、次を満たすときに限る。

- 呼び出し側が責任を予測しやすくなる
- 理解のための往復（薄い facade、無意味なファイル分割）が増えない
- 重複削減だけでなく、認知負荷も減る（[`AGENTS.md`](../../AGENTS.md) レビュー基準と一致）

### 4. メトリクス

- 行数・ファイル数・循環的複雑度・認知的複雑度・Maintainability Index を、
  **単独の合否ゲートにしない**。
- これらは hotspot 抽出やレビュー優先度のための**危険信号**として使ってよい。
- 信号を CI fail や必須閾値にする場合は、本 ADR ではなく別の plan / issue で
  根拠・段階導入・誤検知対策を定める。

---

## 代替案（却下）

1. **SOLID を常時全面適用する** — 早期の interface / 層が増え、YAGNI と衝突し、
   小規模変更の認知負荷が上がる。
2. **DRY をコピペ禁止として厳格適用する** — 偶然似た形のコードまで無理に共通化し、
   無関係な変更が結合する。
3. **複雑度しきい値を直ちに CI fail にする** — 既存コードと正当な分岐密度の多い
   経路（parser / dispatch）を一斉に罰し、導入が失敗しやすい。信号用途に留める。

---

## 帰結（採択時）

- 設計レビューとエージェント実装は「まず単純解、知識重複が育ってから抽象化」を既定にする。
- [`docs/process/coding-conventions.md`](../process/coding-conventions.md) の層所有や
  API 形（trait / method）とは直交する。そちらは言語・コンパイラの拘束であり、
  本 ADR は「いつ抽象化するか」の順序である。
- 品質ツール分業は [ADR-047](ADR-047-code-quality-tooling-and-gates.md) が担当する。

## 再検討条件

- マルチパッケージ / 公開ライブラリ境界が増え、DIP 等を初期から強制した方が
  壊れにくいと実証されたとき。
- 複雑度信号の運用データが揃い、限定パスでの閾値 fail が誤検知より価値が高いと
  示されたとき（別 ADR または plan で閾値を提案する）。

## 関連

- [`docs/research/code-quality-practices.md`](../research/code-quality-practices.md)
- [ADR-047](ADR-047-code-quality-tooling-and-gates.md)（提案）
- [`AGENTS.md`](../../AGENTS.md)
- [`docs/process/coding-conventions.md`](../process/coding-conventions.md)
