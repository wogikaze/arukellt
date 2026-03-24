# v0 スコープ定義

## 概要

arukellt v0 は「LLM フレンドリな言語」の最小限の実装。
コア機能に集中し、高度な機能は v1 以降に持ち越す。

---

## v0 に含めるもの

### 言語機能

| 機能 | 状態 |
|------|------|
| プリミティブ型（i32, i64, f32, f64, bool, char） | ✅ |
| struct | ✅ |
| enum（タグ付き union） | ✅ |
| パターンマッチ（match） | ✅ |
| ジェネリック関数（制限付き） | ✅ |
| Option[T], Result[T, E] | ✅ |
| if/else, while, loop | ✅ |
| ? 演算子（エラー伝播） | ✅ |
| モジュールシステム | ✅ |
| クロージャ | ✅ |
| 基本演算子（算術、比較、論理） | ✅ |

### 標準ライブラリ

| モジュール | 状態 |
|-----------|------|
| core/mem | ✅ |
| core/option | ✅ |
| core/result | ✅ |
| collections/string | ✅ |
| collections/vec | ✅ |
| io/fs | ✅ |
| io/clock | ✅ |
| io/random | ✅ |

### ツールチェイン

| ツール | 状態 |
|--------|------|
| arukellt compile | ✅ |
| arukellt run | ✅ |
| 複数エラーの一括報告 | ✅ |
| Wasm バイナリ出力 | ✅ |
| WASI p1 サポート | ✅ |

---

## v0 に含めないもの

### 言語機能

**注**: `break` / `continue` は v0 に含まれている。

| 機能 | 理由 | 予定 | v1 優先度 |
|------|------|------|----------|
| for 構文（限定版） | trait 不要。範囲 `0..n` + Vec 走査 `values(v)` | v1 | **P1** |
| 文字列補間 `f"..."` | concat ネスト解消 | v1 | **P2** |
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
| WASI p2 サポート | Component Model 対応が必要 | v1 |
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
   - ファイル読み書き
   - 数値計算（フィボナッチ、素数判定）
   - エラー処理（Result の連鎖）
3. Wasm バイナリサイズが許容範囲内
   - Hello World: 5KB 以下
4. 複数エラーの一括報告が機能

---

## 関連

- ADR-002: メモリモデル（Wasm GC）
- ADR-003: ジェネリクス戦略
- ADR-004: trait 戦略（v0 では trait なし）
- `docs/stdlib/README.md`: 標準ライブラリ追加順序
