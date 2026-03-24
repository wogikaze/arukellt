# Wasm 機能の 3 層分類

ADR-002 により **Wasm GC 前提** で設計する。

---

## 概要

arukellt が使用する Wasm 機能を 3 層に分類する。

- **Layer 1 (Baseline)**: v0 で必須
- **Layer 2 (Optional)**: v0 では使用可能だが必須ではない
- **Layer 3 (Future)**: v1 以降で検討

---

## Layer 1: Baseline（v0 必須）

### Core Wasm 1.0

| 機能 | 用途 |
|------|------|
| i32/i64/f32/f64 | プリミティブ型 |
| 関数定義・呼び出し | 基本 |
| 制御フロー (block/loop/if/br) | 構造化制御 |
| ローカル変数 | 関数内変数 |
| グローバル変数 | 定数・状態 |

### Multi-value

| 機能 | 用途 |
|------|------|
| 複数戻り値 | tuple, Result の lowering |
| ブロックの複数結果 | 効率的な制御フロー |

### Reference Types

| 機能 | 用途 |
|------|------|
| `funcref` | 関数参照 |
| `externref` | 外部参照（FFI） |
| `ref.null` | null 参照 |
| `ref.is_null` | null チェック |

### GC Types（Wasm GC）

| 機能 | 用途 |
|------|------|
| `struct` | ユーザー定義構造体 |
| `array` | 動的配列 |
| `ref` / `ref null` | 参照型 |
| `struct.new` / `struct.get` / `struct.set` | 構造体操作 |
| `array.new` / `array.get` / `array.set` / `array.len` | 配列操作 |
| `ref.cast` / `ref.test` | 型検査・キャスト |

### Linear Memory

| 機能 | 用途 |
|------|------|
| `memory` | WASI 用バッファ |
| `i32.load` / `i32.store` 等 | メモリアクセス |

### WASI Preview 1

| 機能 | 用途 |
|------|------|
| `fd_write` | 標準出力 |
| `fd_read` | 標準入力 |
| `path_open` | ファイル操作 |
| `clock_time_get` | 時刻取得 |
| `random_get` | 乱数取得 |
| `proc_exit` | プロセス終了 |

---

## Layer 2: Optional（v0 使用可能）

### Bulk Memory Operations

| 機能 | 用途 |
|------|------|
| `memory.copy` | バッファコピー |
| `memory.fill` | バッファ初期化 |

### Mutable Globals (import)

| 機能 | 用途 |
|------|------|
| 可変グローバルの import | ホストとの状態共有 |

### Sign Extension

| 機能 | 用途 |
|------|------|
| `i32.extend8_s` 等 | 符号拡張 |

---

## Layer 3: Future（v1 以降）

### Exception Handling

| 機能 | 用途 |
|------|------|
| `try` / `catch` / `throw` | 例外ベースエラー |

**方針**: v0 では Result ベースエラー処理のみ。Exception Handling は panic の unwinding に検討。

### Tail Call

| 機能 | 用途 |
|------|------|
| `return_call` | 末尾呼び出し最適化 |

**方針**: v0 では未使用。関数型スタイルの最適化として v1 で検討。

### Threads

| 機能 | 用途 |
|------|------|
| 共有メモリ | 並行処理 |
| アトミック操作 | 同期 |

**方針**: v0 では未使用。async 設計後に検討。

### Component Model

| 機能 | 用途 |
|------|------|
| WIT インターフェース | 型安全な FFI |
| Component linking | モジュール結合 |

**方針**: WASI Preview 2 対応時に検討。ADR-006 に従い Layer 2 の拡張として扱う。

### SIMD

| 機能 | 用途 |
|------|------|
| `v128` | ベクトル演算 |

**方針**: v0 では未使用。数値計算最適化として将来検討。

---

## ランタイム要件

### v0 必須ランタイム

Wasm GC をサポートするランタイム:

- wasmtime 28.0+ (`--wasm gc` フラグ)
- V8 (Chrome 119+)
- SpiderMonkey (Firefox 120+)

### 非サポートの扱い

Wasm GC 非対応ランタイムでは arukellt の Wasm は動作しない。フォールバックは提供しない。

---

## 関連

- ADR-002: GC 採用の決定
- `docs/abi.md`: ABI 表現
- `docs/language/memory-model.md`: メモリレイアウト詳細
