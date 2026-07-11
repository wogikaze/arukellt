# ADR-018: 言語ドキュメント分類 — Normative / Explanatory / Transitional

ステータス: **ACCEPTED** — 3つのドキュメントクラス（normative/explanatory/transitional）を採用
作成日: 2026-04-14
範囲: Language documentation (`docs/language/`), docs tooling

---

## 文脈

`docs/language/` には目的の異なる文書が混在している。

- 実装済み・fixture 裏付けの挙動を権威的に定めるもの（例: `spec.md`、`syntax.md`）
- 挙動の正本ではなく概念や使い方を説明するもの
- まだ完全に着地していない計画中・進行中の変更を記述するもの（例: `syntax-v1-preview.md`）

形式的な分類がなければ、読者は文書が挙動を定義するのか、説明するのか、作業中なのかを
一目で判断できない。結果として:

- 移行中ドキュメントを権威として扱う
- 既存文書を更新するか新規作成するかの判断が貢献者ごとにばらつく
- 移行中ドキュメントをいつ退役・昇格させるかのライフサイクル規則がない

ADR-014 は仕様の*節*と stdlib の*エントリ*に安定性ラベルを付けた。本 ADR は言語 docs
ディレクトリ向けの、文書単位の補完的分類を定める（機能単位ではなく文書単位）。

---

## 決定

### 3 つのドキュメントクラス

| クラス | 意味 | fixture 裏付け必須? | 変更プロセス |
|--------|------|---------------------|--------------|
| **normative** | 実装済み・検証可能な挙動の権威仕様。主張は fixture harness で検証される。 | Yes（または明示的免除） | 変更には仕様レビューが必要 |
| **explanatory** | チュートリアル・概念・利用向け。挙動を説明するが定義しない。normative を参照してよい。 | No | 通常 PR。normative と矛盾してはならない |
| **transitional** | 未着地の計画・進行中変更。着地後に normative へ昇格、既存 normative へ統合、または退役する。 | No（昇格まで） | 退役/卒業条件を示す `DONE_WHEN` が必須 |

### バナーテンプレート

`docs/language/` の各文書は、タイトル見出しの直後にちょうど 1 つの分類バナーを
付けなければならない（MUST）。テンプレートは以下。

#### Normative バナー

```markdown
> **Normative**: This document defines the authoritative behavior of Arukellt as implemented.
> Behavior described here is verified by the fixture harness. Changes require spec review.
> For current verified state, see [../current-state.md](../current-state.md).
```

#### Explanatory バナー

```markdown
> **Explanatory**: This document explains concepts and usage patterns.
> It is not the authoritative specification. For normative behavior, see [../language/spec.md](../language/spec.md)
> and [../current-state.md](../current-state.md).
```

#### Transitional バナー

```markdown
> **Transitional**: This document describes planned or in-progress changes to Arukellt.
> It will be promoted to normative, merged, or retired when the feature lands.
> For current behavior, see [../current-state.md](../current-state.md) and [../language/spec.md](../language/spec.md).
> DONE_WHEN: <condition under which this document graduates or is retired>
```

（バナー本文は既存ツール・生成物との互換のため英語テンプレートを正とする。）

### 言語 docs の分類

| ファイル | クラス | 根拠 |
|----------|--------|------|
| `spec.md` | normative | 凍結された権威仕様。fixture 裏付け。凍結後変更は ADR 必須 |
| `syntax.md` | normative | current-first 構文参照。実装・試験済み挙動を反映 |
| `error-handling.md` | normative | current-first エラー処理。実装済み `Result`/`Option` を反映 |
| `memory-model.md` | normative | current-first メモリモデル。GC-native T3 実装を反映 |
| `type-system.md` | normative | current-first 型システム。実装済み型検査を反映 |
| `syntax-v1-preview.md` | transitional | 未 normative の v1 構文追加。`spec.md` 着地で退役 |

---

## 帰結

- `docs/language/` に追加する新規文書は、マージ前に `docs/data/language-doc-classifications.toml` でクラスを宣言しなければならない（MUST）
- 生成される `docs/language/README.md` は TOML に基づく分類表を示す
- 正式退役なしに後継された transitional は「stale transitional」とみなし、将来の harness 検査対象になりうる
- explanatory は fixture 裏付け検証の対象外である。これは欠陥ではなく意図した性質である

---

## 検討した代替案

**文書ごとに ADR-014 ラベルを流用する単一安定性ラベル**
却下: ADR-014（`stable`/`provisional`/`experimental`）は*機能*の実装成熟度。ここは文書の
*認識論的役割*（権威 / 概念 / 進行中）。直交する — `stable` 機能にも `explanatory` チュートリアルがあり、
`transitional` 設計メモは ADR-014 の意味での「experimental」ではない。

**形式分類なし。既存の blockquote バナーに頼る**
却下: 既存バナーは不統一（`Current-first`、`Transitional`、`Frozen for release`）。クラスと
テンプレートの規範リストがなければ不統一は増える。TOML 正本は機械可読で、将来の harness 検査を可能にする。

---

## 参照

- `docs/language/` — 言語ドキュメントディレクトリ
- `docs/data/language-doc-classifications.toml` — 機械可読な分類データ
- ADR-014: 言語仕様と Stdlib API の安定性ラベル
- `scripts/gen/generate-docs.py` — 分類表付き `docs/language/README.md` を生成
