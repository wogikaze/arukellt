# v0 スコープ定義

## 概要

arukellt v0 は「LLM フレンドリな言語」の最小限の実装。
コア機能に集中し、高度な機能は v1 以降に持ち越す。

> **実装状況の詳細は [`docs/process/v0-status.md`](v0-status.md) を参照。**
> 本文書はスコープ定義（何を含めるか）であり、実装完了の宣言ではない。

凡例: ✅ = end-to-end 動作確認済み, ⚠️ = 部分実装, 🔲 = 設計済み・未実装

---

## v0 に含めるもの

### 言語機能

| 機能 | 状態 | 備考 |
|------|------|------|
| プリミティブ型（i32, bool） | ✅ | 完全動作 |
| プリミティブ型（i64, f32, f64, char） | ⚠️ | リテラル出力可。型固有の演算・変換ヘルパー未実装 |
| String | ✅ | リテラル、String_from、eq、println 動作 |
| struct | ✅ | 定義・初期化・フィールドアクセス動作 |
| enum（unit バリアント） | ✅ | 整数タグとして動作、match 対応 |
| enum（payload バリアント） | 🔲 | Some(T), Err(E) 等のペイロード未実装 |
| パターンマッチ（match） | ⚠️ | int/bool/enum(unit)/wildcard 動作。payload binding 未実装 |
| ジェネリック関数（制限付き） | 🔲 | パース・型検査のみ。monomorphization 未実装 |
| Option\<T\>, Result\<T, E\> | 🔲 | 型登録済み。Some/None/Ok/Err はペイロード未対応 |
| if/else, while, loop | ✅ | 文・式両方、break/continue 対応 |
| ? 演算子（エラー伝播） | 🔲 | パースのみ。型検査・lowering 未実装 |
| モジュールシステム | 🔲 | import 構文パース済み。名前解決・モジュール読込未実装 |
| クロージャ | 🔲 | パースのみ。型検査・lowering・コード生成未実装 |
| 基本演算子（算術、比較、論理） | ✅ | i32 で完全動作。短絡評価対応 |

### 標準ライブラリ

| モジュール | 状態 | 備考 |
|-----------|------|------|
| println / print / eprintln | ✅ | WASI fd_write 経由 |
| i32_to_string / bool_to_string | ✅ | Wasm ヘルパー関数として実装 |
| String_from / eq | ✅ | 文字列生成・比較 |
| core/mem | 🔲 | 設計済み・未実装 |
| core/option | 🔲 | 型登録済み。unwrap 等の関数未実装 |
| core/result | 🔲 | 型登録済み。unwrap 等の関数未実装 |
| collections/string | ⚠️ | concat, slice, split, join 等は名前解決のみ。実行不可 |
| collections/vec | 🔲 | Vec_new_i32 等は名前解決のみ。Vec ランタイム未実装 |
| io/fs | 🔲 | 設計済み・未実装 |
| io/clock | 🔲 | 設計済み・未実装 |
| io/random | 🔲 | 設計済み・未実装 |

### ツールチェイン

| ツール | 状態 | 備考 |
|--------|------|------|
| arukellt compile | ✅ | .wasm ファイル出力 |
| arukellt run | ✅ | wasmtime 組み込みで実行 |
| arukellt check | ✅ | パース + 名前解決 + 型検査 |
| 複数エラーの一括報告 | ✅ | DiagnosticSink + ariadne |
| Wasm バイナリ出力 | ✅ | WASI Preview 1 互換 |
| WASI p1 サポート | ✅ | fd_write のみ。wasm32 ターゲットは未分離 |
| WASI p2 サポート（Component Model / WIT） | 🔲 | 設計済み・未実装 |

---

## v0 に含めないもの

### 言語機能

**注**: `break` / `continue` は v0 に含まれている。

| 機能 | 理由 | 予定 | v1 優先度 |
|------|------|------|----------|
| for 構文（限定版） | trait 不要。範囲 `0..n` + Vec 走査 `values(v)` | v1 | **P1** |
| 文字列補間 `f"..."` | concat ネスト解消。**プリミティブ型のみ**（`i32`, `i64`, `f32`, `f64`, `bool`, `char`, `String`）。カスタム型は P3（Display）が必要 | v1 | **P2** |
| trait / interface | LLM が壊しやすい解決規則。組み込み反復で先に橋渡し | v1 | P3 |
| impl / メソッド構文 | trait 後に導入 | v1 | P4 |
| 演算子オーバーロード | trait が必要 | v1 | P5 |
| ネストしたジェネリクス | コード爆発を防ぐ | v1 | 未定 |
| ユーザー定義 generic struct | v0 の複雑さを抑える | v1 | 未定 |
| async/await | 非同期ランタイム設計が必要 | v2 | — |
| マクロ | 設計が必要 | v2 | — |

### 標準ライブラリ

| モジュール | 理由 | 予定 |
|-----------|------|------|
| iter | Iterator trait が必要 | v1 |
| collections/hashmap | Eq, Hash trait が必要 | v1 |
| fmt | Display trait が必要 | v1 |
| net | async 設計前には入れない | v2 |

### ツールチェイン

| ツール | 理由 | 予定 |
|--------|------|------|
| LLVM バックエンド | Wasm 優先 | v1 |
| LSP サーバー | v0 後に追加 | v1 |
| インクリメンタルコンパイル | v0 後に追加 | v1 |

---

## 制限事項

### ジェネリクスの制限

- 型パラメータは 2 個まで
- ネスト禁止: `Vec[Vec[T]]` は使用不可
- generic struct はライブラリ提供のみ

### クロージャの制限

- キャプチャは値コピーのみ
- mutable capture は `ref` 経由

### エラー処理の制限

- ? 演算子はエラー型が一致する場合のみ
- From trait による自動変換は v1 以降

---

## 成功基準

v0 完成の条件:

1. `scripts/verify-harness.sh` が成功
2. 以下のサンプルがコンパイル・実行可能:
   - Hello World
   - 数値計算（フィボナッチ、素数判定）
   - 構造体・enum の基本操作
   - エラー処理（Result の連鎖）— **ペイロード variant 実装後**
   - ファイル読み書き — **io/fs 実装後**
3. Wasm バイナリサイズが許容範囲内
   - Hello World: 5KB 以下
4. 複数エラーの一括報告が機能

> **現在の到達度**: Hello World + 数値計算 + 構造体は動作。
> ファイル I/O と Result 連鎖は未到達。詳細は `v0-status.md` 参照。

---

## 関連

- ADR-002: メモリモデル（Wasm GC）
- ADR-003: ジェネリクス戦略
- ADR-004: trait 戦略（v0 では trait なし）
- `docs/stdlib/README.md`: 標準ライブラリ追加順序
