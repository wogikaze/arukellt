# Arukellt 設計書

## この設計書について

設計の思想と決定事項を記録する。次の検討者が「何が決まっているか」を把握できることを目的とする。

**ステータス**: v0 コア設計完了（2026-03-24）

---

## ディレクトリ構造

```
arukellt/
│
├── README.md                        # ← 今ここ。全体の入口
│
├── docs/
│   ├── adr/                         # 意思決定ログ（ADR形式）
│   │   ├── ADR-0001-harness-bootstrap.md
│   │   ├── ADR-002-memory-model.md      # ✅ 決定: Wasm GC 採用
│   │   ├── ADR-003-generics-strategy.md # ✅ 決定: 制限付き monomorphization
│   │   ├── ADR-004-trait-strategy.md    # ✅ 決定: v0 では trait なし
│   │   ├── ADR-005-llvm-scope.md        # ✅ 決定: LLVM は Wasm に従属
│   │   └── ADR-006-abi-policy.md        # ✅ 決定: 3層構造
│   │
│   ├── language/                    # 言語仕様
│   │   ├── memory-model.md          # ✅ Wasm GC 前提のメモリモデル
│   │   ├── type-system.md           # ✅ 型システム詳細
│   │   └── syntax.md                # ✅ 構文仕様
│   │
│   ├── platform/                    # Wasm / WASI / ランタイム層
│   │   └── wasm-features.md         # ✅ Wasm 機能の3層分類
│   │
│   ├── stdlib/                      # 標準ライブラリ
│   │   └── README.md                # ✅ 追加順序確定
│   │
│   ├── process/                     # 開発プロセス
│   │   ├── benchmark-plan.md        # ベンチマーク仕様
│   │   ├── benchmark-results.md     # ベンチマーク結果（暫定決定）
│   │   └── v0-scope.md              # ✅ v0 スコープ定義
│   │
│   ├── abi.md                       # 公開 ABI 方針
│   ├── compiler-phases.md           # ✅ コンパイルパイプライン詳細
│   ├── core.md                      # ✅ std/core API
│   ├── decision-guide.md            # 意思決定ガイド
│   ├── error-handling.md            # エラー処理方針
│   ├── io.md                        # ✅ std/io API
│   └── wasi-resource-model.md       # ✅ WASI 資源モデル
│
├── harness/
│   └── proto/                       # ベンチマーク用プロトタイプ
│       ├── gc/                      # Wasm GC 版
│       ├── linear/                  # linear memory 版
│       └── run_bench.sh             # ベンチマーク実行スクリプト
│
├── issues/
│   ├── open/                        # 未解決課題
│   └── done/                        # 解決済み課題
│
└── scripts/
    └── verify-harness.sh            # 検証ハーネス
```

---

## v0 主要決定事項

| ADR | 決定内容 |
|-----|---------|
| ADR-002 | **Wasm GC を採用**。ライフタイム管理なし。LLM フレンドリ優先。 |
| ADR-003 | **制限付き monomorphization**。ネスト禁止、型パラメータ2個まで。 |
| ADR-004 | **v0 では trait なし**。iter/HashMap/for構文は v1 以降。 |
| ADR-005 | **LLVM は Wasm 意味論に従属**。未最適化でよい。 |
| ADR-006 | **ABI は3層まで**。内部/Wasm公開/native公開。 |

---

## 設計の核心思想（確定）

- **WASM32 が主。LLVM IR は従。** 言語の意味論は WASM 側に合わせる。
- **null なし。** Option/Result を徹底する。
- **サブタイピングなし。** 和型 + パターンマッチで代替する。
- **WASI capability を型に直接乗せない。** DirCap + RelPath 方式を採用。
- **LLMフレンドリ = 省略規則が少なく、解決規則が単純。**

---

## v0 スコープ

### 含めるもの

- プリミティブ型、struct、enum、パターンマッチ
- 制限付きジェネリクス、Option/Result
- if/else、while、loop、?演算子
- std: mem, option, result, string, vec, fs, clock, random

### 含めないもの（v1以降）

- trait / interface
- for 構文（Iterator trait が必要）
- 演算子オーバーロード
- iter, HashMap
- async/await

詳細: `docs/process/v0-scope.md`

---

## 次のステップ

1. コンパイラ実装の開始
   - Lexer → Parser → Name Resolution → Type Checker → MIR → Wasm Emitter
2. 標準ライブラリ実装
   - Phase 1: mem, option, result
   - Phase 2: string, vec, fs, clock, random
3. ベンチマーク検証
   - wasmtime 環境でのプロトタイプ実行
   - 性能問題があれば ADR-002 を再検討

---

## 検証

```bash
./scripts/verify-harness.sh
```

すべてのチェックが通れば v0 設計は完了。
