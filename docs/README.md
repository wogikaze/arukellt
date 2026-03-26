# Arukellt

**Wasm-first・静的型付け・LLM-friendly** な言語。

> **v1 実装中** — 154 fixtures pass, M3〜M8 完了（trait/impl/generics/match extensions）

---

## Hello World

```
fn main() {
    println("Hello, world!")
}
```

```bash
arukellt run hello.ark
```

---

## 特徴

| 特徴 | 説明 |
|------|------|
| **Wasm-first** | WebAssembly をプライマリターゲットに設計。ブラウザ・AtCoder・サーバーサイドで動作 |
| **静的型付け** | 型推論付き。コンパイル時に型エラーを検出 |
| **LLM-friendly** | 正規形が少なく、LLM による生成・修正が安定しやすい構文 |
| **Rust-like 構文** | `struct`/`enum`/`match`/`Result`/`Option` — Rust に慣れた開発者には直感的 |
| **Capability I/O** | I/O は `main(caps: Capabilities)` 経由。サンドボックス安全性を保証 |

---

## ミニツアー

### 型と変数

```
let x: i32 = 42
let mut count = 0
let name: String = String_from("Arukellt")
```

### 構造体とメソッド（v1）

```
struct Point { x: i32, y: i32 }

impl Point {
    fn distance(self, other: Point) -> f64 {
        let dx = self.x - other.x
        let dy = self.y - other.y
        sqrt(i32_to_f64(dx * dx + dy * dy))
    }
}

fn main() {
    let p = Point { x: 0, y: 0 }
    let q = Point { x: 3, y: 4 }
    println(f64_to_string(p.distance(q)))  // 5.0
}
```

### パターンマッチ（v1 拡張）

```
enum Shape { Circle(f64), Rect(i32, i32) }

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) if r > 0.0 => 3.14 * r * r,
        Shape::Rect(w, h)           => i32_to_f64(w * h),
        _                           => 0.0,
    }
}
```

### エラー処理

```
fn parse_positive(s: String) -> Result<i32, String> {
    let n = parse_i32(s)
    if n < 0 {
        return Err(String_from("negative value"))
    }
    Ok(n)
}

fn main() {
    match parse_positive(String_from("42")) {
        Ok(n)  => println(i32_to_string(n)),
        Err(e) => println(e),
    }
}
```

---

## ドキュメント

### はじめる

- [クイックスタート](quickstart.md) — 10分で書き始めるガイド

### 言語仕様

- [構文リファレンス](language/syntax.md)
- [型システム](language/type-system.md)
- [エラーハンドリング](language/error-handling.md)
- [メモリモデル](language/memory-model.md)
- [v1 構文プレビュー](language/syntax-v1-preview.md)

### 標準ライブラリ

- [標準ライブラリ概要](stdlib/README.md)
- [コアAPI](stdlib/core.md)
- [I/O](stdlib/io.md)
- [Cookbook](stdlib/cookbook.md) — よく使うパターン集

### コンパイラ

- [パイプライン](compiler/pipeline.md) — Lexer → Parser → Resolve → TypeCheck → MIR → Wasm
- [診断システム](compiler/diagnostics.md) — エラーコード・fix-it hint

### プラットフォーム

- [Wasm 機能レイヤー](platform/wasm-features.md)
- [ABI ポリシー](platform/abi.md)
- [WASI リソースモデル](platform/wasi-resource-model.md)

### 仕様

- [v0 統合仕様](spec/v0-unified-spec.md)

### 意思決定ログ（ADR）

| ADR | タイトル | ステータス |
|-----|---------|-----------|
| [ADR-001](adr/ADR-0001-harness-bootstrap.md) | テストハーネス Bootstrap | DECIDED |
| [ADR-002](adr/ADR-002-memory-model.md) | Wasm GC vs 非GC メモリモデル | DECIDED — GC採用 |
| [ADR-003](adr/ADR-003-generics-strategy.md) | Generics 戦略（制限付き mono） | DECIDED |
| [ADR-004](adr/ADR-004-trait-strategy.md) | Trait 導入タイミング | DECIDED — v1で導入 |
| [ADR-005](adr/ADR-005-llvm-scope.md) | LLVM バックエンドのスコープ | DECIDED — Wasm従属 |
| [ADR-006](adr/ADR-006-abi-policy.md) | ABI ポリシー（3層） | DECIDED |
| [ADR-007](adr/ADR-007-targets.md) | コンパイルターゲット分類（T1〜T5） | DECIDED |

### プロセス

- [v0 実装状況](process/v0-status.md) ← **現在の実装の真実はここ**
- [v0 スコープ](process/v0-scope.md)
- [v1 非ゴール](process/v1-non-goals.md)
- [v0 完了レポート](process/v0-completion-report.md)
- [意思決定ガイド](process/decision-guide.md)
- [LLM 可読性計画](process/llm-readiness-plan.md)
- [ベンチマーク結果](process/benchmark-results.md)

### サンプル

- [Gloss Markdown パーサー](sample/parser.ark) — Arukellt で書かれたパーサー実装

---

## ターゲット（ADR-007）

| ターゲット | 説明 | 状態 |
|-----------|------|------|
| **T1** `wasm32-wasi-p1` | AtCoder 特例。No GC, linear memory, WASI p1 | ✅ 現行実装 |
| **T2** `wasm32-freestanding` | ブラウザ・組み込み。Wasm GC, WASI なし | 🔲 設計済み |
| **T3** `wasm32-wasi-p2` | **メインターゲット**。Wasm GC + Component Model + WASI p2 | 🔲 設計済み |
| **T4** `native` | LLVM 経由。Wasm 意味論に従属 | 🔲 将来 |
| **T5** `wasm32-wasi-p3` | WASI p3 対応（仕様確定後） | 🔲 将来 |

---

## 実装状況

| マイルストーン | 内容 | 状態 |
|-------------|------|------|
| v0 | 基本言語機能（struct/enum/match/Result/Vec/String/for/closure） | ✅ 完了 |
| M3 | Bridge HOFs（any_i32, find_i32） | ✅ 完了 |
| M4 | Trait / Impl / メソッド構文 | ✅ 完了 |
| M5 | Inherent impl（trait なしメソッド） | ✅ 完了 |
| M6 | 演算子オーバーロード（impl ベース） | ✅ 完了 |
| M7 | 構文拡張（match guard / or-pattern / struct pattern / field update） | ✅ 完了 |
| M8 | Generics 拡張（generic struct / nested generics / trait bounds） | ✅ 完了 |

> 詳細は [v0-status.md](process/v0-status.md) を参照。

---

## リポジトリ構造

```
arukellt/
├── crates/
│   ├── ark-lexer/        # 字句解析
│   ├── ark-parser/       # 構文解析（再帰降下）
│   ├── ark-resolve/      # 名前解決・prelude注入
│   ├── ark-typecheck/    # 型検査・型推論
│   ├── ark-mir/          # MIR 中間表現への変換
│   ├── ark-wasm/         # Wasm コード生成
│   └── arukellt/         # CLI（compile / run / check）
├── std/
│   └── prelude.ark       # 標準ライブラリ（Ark で記述）
├── tests/fixtures/       # エンドツーエンドテスト
└── docs/                 # このドキュメントサイト
```
