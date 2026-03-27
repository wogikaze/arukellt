# Wasm 機能の 3 層分類

ADR-002 により **言語セマンティクスは Wasm GC ベース** で設計する。

> **⚠️ 現在の実装状況**: 既定ターゲットは **T1 `wasm32-wasi-p1` (linear memory + WASI Preview 1)**。
> T3 `wasm32-wasi-p2` は **実験実装** があり、基本実行経路と Vec/sort 系は動作するが、
> 実体は P1 runtime + 線形メモリ bridge であり、真の Preview 2 / Component Model 実装ではない。
> 以下の分類は設計方針であり、実装完了を意味しない。
> 現在の実装状況は [`docs/current-state.md`](../current-state.md) を参照。

ADR-007 で定義された 5 つの正規ターゲット:

| ターゲット | ティア | 説明 | 実装状況 |
|-----------|--------|------|---------|
| `wasm32-wasi-p1` | T1 | Linear memory + WASI Preview 1 (AtCoder) | **実装済み** |
| `wasm32-freestanding` | T2 | Wasm GC, WASI なし (ブラウザ/組み込み) | 計画中 |
| `wasm32-wasi-p2` | T3 | Wasm GC + WASI Preview 2 + Component Model | 実験実装（P1 runtime bridge） |
| `native` | T4 | LLVM による native バイナリ | 計画中 |
| `wasm32-wasi-p3` | T5 | Wasm GC + WASI Preview 3 (async-first) | 将来 |

> **注**: 旧ターゲット名 `wasm-gc`, `wasm-gc-wasi-p2`, `wasm32` は互換 alias として受理されるが、
> 使用時に非推奨警告 (W0002) が出る。正規名を使用すること。

---

## T3 `wasm32-wasi-p2` ターゲット: 機能層

arukellt が使用する Wasm 機能を 3 層に分類する。

- **Layer 1 (Baseline)**: v0 で必須
- **Layer 2 (Public Surface)**: v0 の公開面ルール
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

### Component Model / WIT（WASI Preview 2）

| 機能 | 用途 |
|------|------|
| WIT interface/world | 型安全な公開インターフェース |
| Canonical ABI | 値の lower/lift |
| Component linking | コンポーネント結合 |

**方針**: v0 で採用。WASI p1 を置き換えるのではなく、Layer 1 の必須機能として併用する。

---

## Layer 2: Public Surface（v0 公開面ルール）

### Layer 2 public surface rule

- Layer 2A: raw Wasm import/export（主に T1 `wasm32-wasi-p1` / AtCoder 互換）
- Layer 2B: Component Model/WIT（主に T3 `wasm32-wasi-p2` 配布面）
- 同一の言語セマンティクスを両面へ落とす

---

## 参考: Optional（v0 使用可能）

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

### SIMD

| 機能 | 用途 |
|------|------|
| `v128` | ベクトル演算 |

**方針**: v0 では未使用。数値計算最適化として将来検討。

---

## ランタイム要件

### v0 必須ランタイム

現行実装は linear memory + WASI Preview 1 のため、以下のランタイムで動作:

- wasmtime（CLI に組み込み済み）
- 任意の WASI Preview 1 対応ランタイム

将来の T3 `wasm32-wasi-p2` ターゲットでは GC 対応ランタイムが必要:

- wasmtime 28.0+ (`--wasm gc` フラグ)
- V8 (Chrome 119+)
- SpiderMonkey (Firefox 120+)

---

## T1 `wasm32-wasi-p1` ターゲット: 使用機能層

AtCoder（wabt 1.0.34 + iwasm 2.4.1）向け。Wasm GC 命令と Component Model は使用しない。

### Layer 1: T1 必須機能

| 機能 | 用途 |
|------|------|
| Core Wasm 1.0（i32/i64/f32/f64、制御フロー、ローカル変数） | 基本演算・制御 |
| Multi-value | tuple / Result の lowering |
| `memory` / load / store 命令 | arena / RC のヒープ操作 |
| WASI Preview 1（fd_write / fd_read / proc_exit） | I/O |
| `funcref` | クロージャ関数ポインタ |

### 使用しない機能（T1 では禁止）

| 機能 | 理由 |
|------|------|
| `struct` / `array` 型（Wasm GC） | iwasm 非対応 |
| `ref` / `ref null`（GC 参照） | iwasm 非対応 |
| `ref.cast` / `ref.test` | iwasm 非対応 |
| Component Model / WIT | iwasm / wabt 非対応 |

### GC 型の lowering 方針（T1 時）

| T3 GC 型 | T1 での表現 |
|-----------|----------------|
| `(ref $struct_T)` | linear memory ポインタ（`i32` オフセット） |
| `(ref $array_T)` | `(i32 ptr, i32 len)` のペア |
| `(ref $string)` | `(i32 ptr, i32 byte_len)` のペア |
| クロージャ | `(i32 fn_ptr, i32 env_ptr)` のペア |
| `ref null` | `i32` の 0（null pointer） |

メモリ管理は ADR-002 補足決定に従い arena + RC hybrid。

---

## 関連

- ADR-002: GC 採用の決定
- `docs/platform/abi.md`: ABI 表現
- `docs/language/memory-model.md`: メモリレイアウト詳細
