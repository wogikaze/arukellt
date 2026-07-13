# ADR-024: Selfhost MIR は SSA 形成前に明示的 CFG を採用する

ステータス: **ACCEPTED** — Selfhost MIRはSSA形成前に明示的なCFGを採用  
作成日: 2026-04-15  
決定日: 2026-04-15
範囲: selfhost MIR, SSA formation, lowering, codegen boundary

## 文脈

Issue #494 は selfhost MIR パイプラインの SSA 形成（合流点での phi 挿入を含む）を担当する。
現行 MIR は SSA が必要とするグラフ構造をまだ提供していない。

- `src/compiler/mir.ark` は `MirBlock` に `succ0` / `succ1` を定義するが、
  HIR→MIR lowering はそれらを埋めない。
- `lower_expr` は `MIR_IF`、`MIR_ELSE`、`MIR_END`、`MIR_BLOCK`、`MIR_LOOP`、
  `MIR_BR_IF` などの構造化マーカーを、関数あたり単一の lowered ブロックへ出す。
- `issues/done/503-selfhost-mir-cfg-infrastructure.md` は、欠落している
  predecessor リスト、即時支配子、支配フロンティア、phi サポートを #494 のブロッカーとして既に記録している。

つまり現行 MIR はパスが足りないだけでなく、標準 SSA アルゴリズムが消費する
明示的 CFG 表現そのものを欠いている。

## 決定

Selfhost MIR は SSA 形成の前に明示的制御フローグラフへ移行しなければならない。

selfhost パイプラインの正準 MIR 表現は次とする。

- 関数あたり複数の基本ブロック
- 条件分岐・無条件分岐の明示的 terminator 辺
- 埋められた successor / predecessor リスト
- ブロック単位の支配情報
- SSA パスが使える支配フロンティア集合
- 合流ブロックに付く phi 表現

構造化制御フローマーカーは一時的な lowering 補助、またはバックエンド固有の再構造化として
残してよいが、SSA パスが消費する正準表現ではない。

## #494 の前に必要な理由

SSA 形成は、現行の構造化のみの lowering が提供しないグラフ情報を必要とする。

1. Phi 挿入は各合流ブロックの predecessor リストを要する。
2. 支配子・支配フロンティア計算は、入れ子の `IF`/`ELSE`/`END` マーカー付き単一命令列ではなく、実 CFG を要する。
3. SSA の変数リネームは明示的ブロック境界と合流点に錨を置く必要がある。

# 494 が現行の構造化 MIR 上に直接 SSA を組むと、まずマーカーから CFG を再構築し、
支配を計算し、phi を入れることになる。同じグラフ作業を SSA パス内で重複させ、
後続解析にとっても表現が曖昧なままになる。

## 根拠

1. **アルゴリズム契約に合う**: 標準 SSA は predecessor・支配子・支配フロンティア付き CFG を前提とする。
2. **表現の曖昧さを除く**: 合流とバックエッジが重要なとき、入れ子マーカーよりブロックグラフの方が推論しやすい。
3. **後続パスを正直にする**: MIR が明示 CFG なら、すべての解析パスが SSA と同じ制御フロー構造を見る。
4. **バックエンドの柔軟性を保つ**: Wasm codegen は emit 時に CFG を Wasm の block/loop/if へ再構造化できる。それは codegen の関心であり MIR 契約ではない。

## 帰結

- #494 は、#503 が CFG 構築・predecessor・支配・支配フロンティア・phi を足すまでブロックされたまま。
- `src/compiler/mir.ark` は明示ブロックグラフ構築を、任意最適化ではなく selfhost MIR lowering 契約の一部として扱う。
- Wasm バックエンドは CFG から構造化 Wasm を emit してよいが、SSA 関連作業の主表現として構造化 MIR マーカーに頼ってはならない。

## 検討した代替案

### A. 構造化 MIR を主表現のままにし、CFG は SSA パス内だけで推論する

却下。同じグラフ再構築問題を #494 に押し込み、制御フロー論理を誤った層で重複させる。

### B. 構造化 MIR に ad hoc な phi 処理を足す

却下。Phi は依然として predecessor を意識した明示的合流点を要するため、名前を付けずに実質 CFG になる。

### C. SSA を避け、構造化 MIR を永久に維持する

却下。#494 は明示的に SSA 形成に依存し、現行コードベースも支配フロンティア基盤を欠落前提条件として挙げている。

## 参照

- `src/compiler/mir.ark`
- `issues/done/503-selfhost-mir-cfg-infrastructure.md`
- `issues/done/494-selfhost-mir-ssa-formation.md`
