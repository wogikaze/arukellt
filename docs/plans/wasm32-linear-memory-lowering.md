# wasm32 linear-memory lowering（tracing fallback）

ステータス: 実装計画（決定記録ではない。collector 方針は ADR-075 が正本）

関連 ADR: ADR-002、ADR-075
日付: 2026-03-25（計画抽出: 2026-07-11）

---

`wasm32`（AtCoder / 非 GC ランタイム向け）への lowering 実装方針。
言語仕様の正本は ADR-002（Wasm GC）。本ファイルは backend 実装計画のみ。

---

### wasm32 プロファイル: メモリ管理モデル

`wasm32` は ADR-075 の非移動 mark-sweep tracing runtime を使う。
arena、RC、cycle reject を target-specific な source semantics として導入しない。

#### runtime の段階導入

1. **recorded bump allocation**: 現行の linear allocation を allocation record として登録する。
2. **明示 root frame**: managed local の入口登録、local update、出口解放を compiler が出力する。
3. **mark-sweep**: root から payload の pointer candidate を追跡し、cycle を含む unreachable graph を free list へ戻す。
4. **managed-value lowering**: #874 で struct/enum/Option/Result/Vec/String/HashMap の descriptor と allocator reuse を接続する。

root update が全 managed local に接続されるまで、collector の自動起動は有効化しない。
この制限は live value の誤回収を防ぐための移行境界であり、#873/#874 の完了時に削除する。

### wasm32 プロファイルが課す言語設計制約

以下は GC 依存かつ lowering 不可能なため、言語仕様から除外する:

| 除外機能 | 理由 |
|---------|------|
| moving compaction / finalizer / weak reference | ADR-075 の初期 runtime 契約に含めない |

---
