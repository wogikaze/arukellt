# ADR-007: コンパイルターゲット整理

ステータス: **DECIDED**

決定日: 2026-03-26

---

## 文脈

arukellt は複数のランタイム・用途向けにコードを生成する必要がある。
ターゲットをランタイム軸で整理すると、言語機能セット（GC有無、WASI版）が不明確になる。
ADR-002（Wasm GC 採用）・ADR-005（LLVM 従属）・ADR-006（ABI 3層）との整合を明示するため、
**「言語機能セット × WASI版」軸** でターゲットを確定する。

---

## 決定

ターゲットを以下の 5 つに確定する。

### T1: wasm32-wasi-p1（AtCoder 特例）

| 項目 | 内容 |
|------|------|
| メモリモデル | Linear memory（No GC） |
| WASI | Preview 1 |
| Component Model | なし |
| ランタイム | iwasm 2.4.1 |
| 主な用途 | AtCoder、競技プログラミング |
| 現状 | **現行 v0 実装** |

**ADR-002 例外**: ADR-002 は Wasm GC 採用を決定しているが、AtCoder が WASI p1 + linear memory 環境に固定されている事実を優先し、このターゲットのみ GC なし linear memory を維持する。

### T2: wasm32-freestanding（ブラウザ JS / 組み込み）

| 項目 | 内容 |
|------|------|
| メモリモデル | Wasm GC（Reference Types） |
| WASI | なし |
| Component Model | なし |
| ランタイム | ブラウザ（主）、wasmtime（デバッグ用のみ） |
| 主な用途 | ブラウザ JS 呼び出し、軽量組み込み |

wasmtime での T2 実行はデバッグ目的に限る。
本来の用途はブラウザ環境（V8/SpiderMonkey の Wasm GC 対応済み）。

### T3: wasm32-wasi-p2（メインターゲット）

| 項目 | 内容 |
|------|------|
| メモリモデル | Wasm GC |
| WASI | Preview 2 |
| Component Model | あり（canonical ABI） |
| ランタイム | wasmtime |
| テスト | <https://wa.dev/mizchi:tmgrammar> |
| 主な用途 | サーバーサイド、クラウド、CLI ツール |

ADR-002（GC 採用）・ADR-006（Layer 2B ABI）の**正規ターゲット**。
言語意味論の基準はこのターゲットで定義する。

### T4: native（LLVM 従属バックエンド）

| 項目 | 内容 |
|------|------|
| バックエンド | LLVM IR |
| プラットフォーム | Linux / Windows / macOS |
| ABI | C ABI（System V AMD64 / Windows x64） |
| 従属 | ADR-005: Wasm 意味論に従属 |
| 主な用途 | ローカルデバッグ、性能比較 |

ADR-005 に従い、LLVM バックエンドは **Wasm 意味論の再現**に留める。
native 専用の言語機能・最適化は追加しない。

### T5: wasm32-wasi-p3（将来ターゲット）

| 項目 | 内容 |
|------|------|
| メモリモデル | Wasm GC |
| WASI | Preview 3 |
| Component Model | あり |
| ランタイム | wasmtime（対応後） |
| テスト | <https://wa.dev/mizchi:tmgrammar> |
| 状態 | 仕様策定中。着手は WASI p3 安定化後 |

T3 の後継。WASI p3 は async-first 設計で、長期的なメインターゲット候補。

---

## ターゲット優先順位

```
実装優先度: T1（現行） → T3（メイン） → T2 → T4 → T5
言語意味論の基準: T3
ADR-002 GC採用: T2, T3, T5（T1 のみ例外）
ADR-005 LLVM従属: T4
ADR-006 ABI 3層: T2/T3 = Layer 2、T4 = Layer 3
```

---

## 禁止事項

- T1 の linear memory 実装を T2/T3 に持ち込まない
- T4（LLVM）で T3 にない言語機能を追加しない（ADR-005）
- T5 への着手は WASI p3 仕様確定前には行わない

---

## 関連

- ADR-002: Wasm GC 採用（T1 の例外根拠）
- ADR-005: LLVM バックエンドの役割制限（T4）
- ADR-006: 公開 ABI 3層構造（T2/T3/T4）
- `docs/platform/abi.md`: ABI 詳細
- `docs/platform/wasm-features.md`: 使用 Wasm 機能
