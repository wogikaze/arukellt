# LLM対応強化フェーズ

言語コア設計（ADR群）の主要決定が揃い、LLM 対応の設計を進めている段階。

---

## 現状認識

**ADR 決定済み**: Wasm GC、generics 方針、trait なし、ABI 等  
**進行中**: LLMが壊しても直せる設計、wasm32 ターゲット設計、未洗い出し要件の比較・段取り  
**未完**: v0 設計全体（freeze 未）

---

## 優先順位（Phase 2）

### 1. 診断（diagnostics）設計 ⭐最重要

**目的**: LLMが壊れたコードを修正できる診断出力

**タスク**:
- [ ] エラーフォーマット固定
  - expected / actual を必ず出す
  - 型エラーは局所で切る（1エラー1原因）
- [ ] fix-it 標準化
  - 例: `missing type` → 「`: T` を追加」
  - 例: `wrong type` → 「`i32` → `i64` に変更」
- [ ] エラー分類を絞る
  - type mismatch
  - unresolved name
  - invalid construct (v0禁止構文)
- [ ] LLM向け例集作成（最重要）
  - 壊れやすいコード → 正しい診断 → 修正後コード

**成果物**:
- `docs/compiler/diagnostics.md` - 診断システム仕様
- `docs/process/llm-error-patterns.md` - LLM向けエラーパターン集

---

### 2. Core API の canonicalization

**目的**: 「正解の書き方」を1個に固定

**対象**: Vec, String, Option/Result, slice `[T]`

**タスク**:
- [ ] 正解パターンの固定
  - 取得: `vec_get(v, i)` / `get(v, i)`
  - 追加: `vec_push(v, x)` / `push(v, x)`
  - 長さ: `len(v)`
- [ ] 禁止パターンの明記
  - `v.push(x)` は書けない（メソッドなし）
  - `v[i] = x` は書けない（インデックス代入なし）
- [ ] 小さな cookbook 作成
  - map/filter 的処理の書き方
  - loop パターン（while only）
  - error handling パターン

**成果物**:
- `docs/stdlib/core.md` 更新
- `docs/stdlib/cookbook.md` 新規作成

---

### 3. Quickstart（最小成功パス）

**目的**: 「これだけ読めば書ける」導線

**内容**:
- hello world
- Vec を使う
- Result を使う
- file read（capability付き）

**要件**:
- すべて v0 canonical style
- 一切の例外を含めない
- import / prelude / capability を明確に

**成果物**:
- `docs/quickstart.md`

---

### 4. v0-unified-spec の最終整理

**目的**: freeze前の最終確認

**タスク**:
- [ ] syntax / type / memory / error の統合確認
- [ ] 用語の統一
- [ ] v0 / v1 境界の明記
- [ ] non-goals を明文化
  - trait なし
  - method なし
  - for なし
  - operator overload なし

**成果物**:
- `docs/spec/v0-unified-spec.md` 更新

---

## （余力）Phase 3

### 5. benchmark / harness 強化

- LLMにコードを書かせるテストセット
- 失敗パターンの収集
- 診断とAPIの調整

---

## 依存関係

```
diagnostics-design
    ↓
core-api-canonical
    ↓
quickstart-guide
    ↓
unified-spec-finalize
```

---

## 移行点

**Before**: 「書ける言語」（文法定義完了）
**After**: 「壊れても直せる言語」（診断・API・ガイド完備）

---

## 関連

- `docs/spec/v0-unified-spec.md` - 統合仕様
- `docs/process/decision-guide.md` - 意思決定ガイド
- `docs/compiler/pipeline.md` - コンパイラパイプライン
