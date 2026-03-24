# ベンチマーク結果

## 実行状況

**ステータス**: プロトタイプ準備完了、ベンチマーク実行保留

**理由**: wasmtime が環境にインストールされていないため実行不可

---

## 準備済みプロトタイプ

| ケース | GC版 | Linear版 |
|--------|------|---------|
| Hello World | `harness/proto/gc/hello.wat` | `harness/proto/linear/hello.wat` |
| Vec push/pop 10k | `harness/proto/gc/vec_pushpop.wat` | `harness/proto/linear/vec_pushpop.wat` |

---

## 暫定決定

`docs/process/decision-guide.md` の暫定決定ルールに従う:

> **LLM フレンドリを最優先に近く置くなら: Wasm GC を選ぶ**

### 根拠

1. **Wado の実績データ**
   - Hello World: 1090 bytes
   - pi_approx: 11786 bytes
   - 実行性能: C 比 1.08–1.23x

2. **LLM フレンドリ性**
   - ライフタイム管理が言語設計から消える
   - borrow checker のような複雑な規則が不要
   - LLM が書き間違える箇所が減る

3. **バイナリサイズ**
   - メモリ管理ルーチンをバイナリに含めなくて済む
   - GC ランタイムはホスト側が提供

4. **実装の単純さ**
   - クロージャ・コンテナの実装が単純
   - 所有権モデルの選択が不要

### リスク

- wasmtime の Wasm GC 最適化は 2025 年時点で未成熟
- ホスト GC の実装品質に性能が左右される
- GC オーバーヘッドがワークロード依存で発生

---

## 決定

**Wasm GC を採用する**

ベンチマーク測定は後続フェーズで検証として実施する。性能問題が判明した場合はフォールバック計画を検討する。

---

## 後続作業

ベンチマーク環境が整った時点で以下を実施:

1. `wasmtime` のインストール（GC サポート有効）
2. `harness/proto/run_bench.sh` の実行
3. 結果をこのファイルに追記
4. 性能問題があれば ADR-002 を再検討
