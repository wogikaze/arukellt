# Arukellt

Wasm-first、LLM-friendly を目指す静的型付け言語。

**Status**: v0 実装完了・v1 機能拡張中（2026-03-27）

> 詳細な実装状況は [docs/current-state.md](docs/current-state.md) を参照。

---

## Quick Links

| ドキュメント | 内容 |
|-------------|------|
| [Quickstart](docs/quickstart.md) | 10分で書き始められるガイド |
| [v0 統合仕様](docs/spec/v0-unified-spec.md) | v0 の完全仕様 |
| [Cookbook](docs/stdlib/cookbook.md) | API 使用パターン集 |
| [診断システム](docs/compiler/diagnostics.md) | エラー診断の仕様 |
| [意思決定ガイド](docs/process/decision-guide.md) | ADR 決定まとめ |

---

## ディレクトリ構造

```
arukellt/
│
├── README.md                        # プロジェクト入口
├── AGENTS.md                        # リポジトリ契約
│
├── docs/
│   ├── quickstart.md                # ⭐ 最初に読むガイド
│   │
│   ├── adr/                         # 意思決定ログ（ADR形式）
│   │   ├── ADR-0001-harness-bootstrap.md
│   │   ├── ADR-002-memory-model.md      # Wasm GC 採用
│   │   ├── ADR-003-generics-strategy.md # 制限付き mono
│   │   ├── ADR-004-trait-strategy.md    # v0 trait なし
│   │   ├── ADR-005-llvm-scope.md        # LLVM は Wasm 従属
│   │   └── ADR-006-abi-policy.md        # 3層 ABI
│   │
│   ├── language/                    # 言語仕様
│   │   ├── memory-model.md          # Wasm GC メモリモデル
│   │   ├── type-system.md           # 型システム
│   │   ├── syntax.md                # 構文仕様
│   │   └── error-handling.md        # エラー処理
│   │
│   ├── compiler/                    # コンパイラ設計
│   │   ├── pipeline.md              # コンパイルパイプライン
│   │   └── diagnostics.md           # ⭐ エラー診断システム
│   │
│   ├── platform/                    # プラットフォーム層
│   │   ├── wasm-features.md         # Wasm 機能の3層分類
│   │   ├── abi.md                   # ABI 方針
│   │   └── wasi-resource-model.md   # WASI 資源モデル
│   │
│   ├── stdlib/                      # 標準ライブラリ
│   │   ├── README.md                # 追加順序
│   │   ├── cookbook.md              # ⭐ 使用パターン集
│   │   ├── core.md                  # core API
│   │   └── io.md                    # I/O API
│   │
│   ├── design/                      # 設計詳細
│   │   ├── gc-mono-tradeoff.md      # GC+mono トレードオフ
│   │   ├── value-semantics.md       # 値セマンティクス
│   │   ├── gc-c-abi-bridge.md       # GC⇔C ABI 境界
│   │   ├── trait-less-abstraction.md # 抽象化戦略
│   │   └── reference-control.md     # 参照制御
│   │
│   ├── spec/                        # 統合仕様
│   │   └── v0-unified-spec.md       # ⭐ v0 完全仕様
│   │
│   └── process/                     # プロセス文書
│       ├── agent-harness.md         # エージェントガイド
│       ├── decision-guide.md        # 意思決定ガイド
│       ├── llm-readiness-plan.md    # LLM 対応計画
│       ├── benchmark-plan.md        # ベンチマーク仕様
│       ├── benchmark-results.md     # ベンチマーク結果
│       └── v0-scope.md              # v0 スコープ
│
├── harness/
│   └── proto/                       # ベンチマーク用プロトタイプ
│       ├── gc/                      # Wasm GC 版
│       ├── linear/                  # linear memory 版
│       └── run_bench.sh             # 実行スクリプト
│
├── issues/
│   ├── open/                        # 未解決課題
│   └── done/                        # 解決済み課題
│
└── scripts/
    └── verify-harness.sh            # 検証ハーネス
```

---

## 言語特徴

### 設計原則

1. **Wasm が正**: Wasm 意味論が唯一の動作定義
2. **簡潔さ優先**: 性能より理解しやすさ
3. **GC 採用**: 所有権/借用の複雑さを回避
4. **制限付き機能**: 必要最小限の言語機能

### Hello World

```
fn main() {
    print("Hello, world!")
}
```

### 特徴的な制約（v0）

- **メソッド構文なし**: `v.push(x)` ではなく `push(v, x)`
- **for ループなし**: `while` のみ（trait 依存のため）
- **trait なし**: 型ごとに関数を提供（`map_i32_i32`, `filter_String` など）
- **ネストしたジェネリクス禁止**: `Vec[Vec[i32]]` は不可

詳細: [Quickstart](docs/quickstart.md)

---

## v0 主要決定事項

| ADR | 決定内容 | 根拠 |
|-----|---------|------|
| [ADR-002](docs/adr/ADR-002-memory-model.md) | **Wasm GC 採用** | LLMフレンドリ、Wado 実績 |
| [ADR-003](docs/adr/ADR-003-generics-strategy.md) | **制限付き mono** | 値型特化、参照型統一 |
| [ADR-004](docs/adr/ADR-004-trait-strategy.md) | **v0 trait なし** | 複雑さ回避 |
| [ADR-005](docs/adr/ADR-005-llvm-scope.md) | **LLVM は Wasm 従属** | Wasm 意味論が正 |
| [ADR-006](docs/adr/ADR-006-abi-policy.md) | **3層 ABI** | 内部/Wasm/native |

---

## v0 スコープ

### 提供する機能

- **型システム**: プリミティブ、struct、enum、tuple
- **制限付き generics**: `Vec[T]`, `Option[T]`, `Result[T, E]`
- **制御構文**: if/else, while, loop, match, ?演算子
- **高階関数**: closure、関数ポインタ
- **標準ライブラリ**: mem, option, result, string, vec, fs, clock, random
- **WASI 公開面**: p1（AtCoder/wasm32）+ p2（Component Model/WIT on wasm-gc）

### 提供しない機能（v1以降）

| 機能 | 理由 |
|------|------|
| trait / interface | 複雑さ回避 |
| impl / メソッド構文 | 関数呼び出しで統一 |
| iterator / for ループ | trait 依存 |
| 演算子オーバーロード | trait 依存 |
| HashMap | trait 依存（Hash, Eq） |
| マクロ | 複雑さ回避 |
| async/await | v0 スコープ外 |

詳細: [v0 スコープ](docs/process/v0-scope.md)

---

## API スタイル

すべて**関数呼び出し形式**（メソッド構文なし）：

```
// Vec操作
let v: Vec[i32] = Vec_new_i32()
push(v, 42)
let x: Option[i32] = get(v, 0)

// String操作
let s1 = String_from("hello")
let s2 = concat(s1, " world")

// ファイルI/O
fn main(caps: Capabilities) -> Result[(), IOError] {
    let dir = cwd(caps)
    let content = fs_read_file(dir, RelPath_from("input.txt")?)?
    print(content)
    Ok(())
}
```

詳細: [Cookbook](docs/stdlib/cookbook.md)

---

## LLM 対応

### Phase 1: 言語設計（完了）

- [x] ADR 決定（GC, mono, trait, LLVM, ABI）
- [x] 型システム設計
- [x] 構文設計
- [x] 標準ライブラリ設計

### Phase 2: LLM-readiness（完了）

- [x] [診断システム](docs/compiler/diagnostics.md): expected/actual、fix-it提案
- [x] [API 正規化](docs/stdlib/cookbook.md): 正解パターンの固定
- [x] [Quickstart](docs/quickstart.md): 最小成功パス
- [x] [統合仕様](docs/spec/v0-unified-spec.md): v0 完全仕様

詳細: [LLM-readiness Plan](docs/process/llm-readiness-plan.md)

---

## 次のステップ

### Phase 3: 実装（完了）

1. **コンパイラ実装** ✅
   - Lexer → Parser → Name Resolution → Type Checker → MIR → Wasm Emitter
   - エラー診断システム実装

2. **標準ライブラリ実装** ✅
   - Phase 1: mem, option, result
   - Phase 2: string, vec
   - Phase 3: fs, clock, random

3. **検証** ✅
   - テストスイート作成（95 unit tests, 182 fixture tests）
   - ベンチマーク実行（wasmtime）
   - LLM 互換性テスト

---

## 検証

設計の完全性を検証：

```bash
./scripts/verify-harness.sh
```

すべてのチェックが通れば v0 設計は完了。

---

## 関連リンク

- [Wado](https://github.com/orsinium-labs/wado) - Wasm GC の実績
- [WASI Preview 1](https://github.com/WebAssembly/WASI)
- [Wasm GC Proposal](https://github.com/WebAssembly/gc)
