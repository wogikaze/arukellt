# 意思決定ガイド

v0 設計の主要決定は完了。このガイドは決定内容の要約と、今後の判断基準を示す。

---

## 決定済み ADR 一覧

| ADR | 決定 | 根拠 |
|-----|------|------|
| ADR-002 | **Wasm GC 採用** | LLMフレンドリ、Wadoの実績 |
| ADR-003 | **制限付き mono** | 値型特化、参照型統一表現 |
| ADR-004 | **v0 trait なし** | 複雑さ回避、v1で導入 |
| ADR-005 | **LLVM は Wasm 従属** | Wasm意味論が正 |
| ADR-006 | **3層ABI** | 内部/Wasm/native |

---

## 現在のフェーズ

**「思想フェーズ」完了 → 「仕様を潰すフェーズ」完了 → 「実装フェーズ」へ**

実装時に新しい設計判断が必要になった場合は、ADR を追加する。

---

## v0 実装の優先順位

以下の順で実装を進める:

1. **パーサ → 名前解決 → 型検査 → Wasm emit（数値計算）**
   - 最小の縦一本を先に通す

2. **struct / enum / match**
   - 型システムの基礎

3. **Vec / String / Option / Result**
   - GC 前提で実装

4. **closure / 高階関数**
   - 抽象化手段として重要

5. **WASI I/O（fs, clock, random）**
   - DirCap + RelPath 方式

---

## 判断に迷ったときのチェックリスト

```
□ Wasm 意味論に反していないか?
  → 反している: ADR-005 を再読。LLVM/native の都合で言語を変えない

□ v0 スコープに trait / iter / HashMap / async を追加しようとしていないか?
  → Yes: docs/process/v0-scope.md の「入れないもの」を再読

□ GC + mono のトレードオフを考慮しているか?
  → docs/design/gc-mono-tradeoff.md を参照

□ 値セマンティクスのルールに従っているか?
  → docs/design/value-semantics.md を参照

□ 「両対応」の誘惑に負けていないか?
  → 一択に絞る。両対応は二重実装になる
```

---

## 関連ドキュメント

| ドキュメント | 内容 |
|-------------|------|
| `docs/spec/v0-unified-spec.md` | v0 統合仕様書 |
| `docs/adr/` | 各 ADR 決定 |
| `docs/design/` | 設計詳細（トレードオフ、セマンティクス等） |
| `docs/process/v0-scope.md` | v0 スコープ |
