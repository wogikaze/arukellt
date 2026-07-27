# Selfhost phase arena ownership (compiler bump)

ステータス: 設計決定メモ（製品コードなし）  
日付: 2026-07-25  
関連: [#827](../../issues/done/827-phase-arena-after-heap-model.md)、[#730](../../issues/done/730-bootstrap-wasm-4gb-memory-limit.md)、[#823](../../issues/done/823-selfhost-compile-latency-quadratic-mir.md)、[#843](../../issues/open/843-wasm32-gc-bootstrap-pin.md)、[ADR-002](../adr/ADR-002-memory-model.md)

## スコープ

この文書は **セルフホストコンパイラの bump / phase arena** だけを扱う。

- ADR-002（言語意味論の Wasm GC）は変更しない。
- `src/compiler/**` に Arena 実装を入れない（#827 受け入れどおり、決定後も本メモだけでは製品コード禁止）。

## 三つの決定

### 1. Phase lifetime

Arena は compile session 内の phase 単位で所有する。

| Phase | 役割の例 |
|-------|----------|
| parse | AST / token 一時バッファ |
| typecheck | 推論スクラッチ、一時制約 |
| lower | MIR 構築中の一時 Vec / map |
| emit | Wasm 書き込みスクラッチ |

**reset が合法なのは**「その phase の消費者が終了した境界」だけである。  
phase 途中の speculative reset や、次 phase がまだ読むデータがある状態での reset は禁止する。

### 2. Cross-arena references

Resettable arena 同士の **生参照は禁止**する。

跨ぎが必要な値は次のいずれかだけ許す。

- session-durable な handle（index into a durable table）
- 明示的に durable bump / session heap へコピーした所有データ

「一時ポインタを後続 phase に持ち越す」は禁止。

### 3. Ownership of data that survives into final Wasm

最終 Wasm バイト列と、後続 phase が読む永続テーブル（型表、シグネチャ、export 名など）は、  
**reset されない durable bump**（現行の「コンパイル終了まで reset しない bump」を含む）に置く。

phase-resettable arena に置いてはならない。

## 計測との紐付け

プロトタイプを始める場合の効果測定は次に紐づける。

- [#823](../../issues/done/823-selfhost-compile-latency-quadratic-mir.md) の wall / RSS 方針
- `.build/selfhost/selfhost-latency-receipt.json`（#829）および同等の phase receipt

Memory64（#730）は OOM 回避であり、bump 未回収そのものの代替ではない。

## Scoped prototype plan（実装は別 issue）

本メモの受け入れ後でも、製品コードは別 issue を切るまで書かない。計画だけ固定する。

1. **Reset points（候補）**: parse→typecheck、typecheck→lower、lower→emit。各境界で「durable へ昇格済み」を検査。
2. **Verify gates**: `verify lane`、selfhost compile smoke、`selfhost fixpoint`（pin 経路変更時）、RSS/wall receipt 比較。
3. **Non-goals**: `#824` early body lowering への arena 結合、言語 GC ヒープとの統合。

## ADR-002 との境界

ADR-002 はユーザープログラムのメモリモデル（Wasm GC）を採択する。  
コンパイラ bootstrap の bump / phase reset は言語意味論の外であり、本メモを正とする。
