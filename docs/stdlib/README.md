# 標準ライブラリ設計方針

> **実装状況**: v0 コンパイラの stdlib は Wasm ヘルパー関数としてコード生成に組み込み。
> 以下の API が end-to-end 動作:
>
> **出力**: `println`, `print`, `eprintln`（i32/i64/f64/bool/String 自動対応）
> **変換**: `i32_to_string`, `i64_to_string`, `f64_to_string`, `bool_to_string`, `parse_i32`, `parse_i64`, `parse_f64`
> **String**: `String_from`, `String_new`, `eq`, `concat`, `slice`, `split`, `join`, `is_empty`, `starts_with`, `ends_with`
> **Vec**: `Vec_new_i32`, `Vec_new_String`, `push`, `pop`, `get`, `set`, `len`, `sort_i32`
> **Vec 高階**: `map_i32_i32`, `filter_i32`, `fold_i32_i32`
> **Option**: `unwrap`, `unwrap_or`, `is_some`, `is_none`
> **数学**: `sqrt`, `abs`, `min`, `max`
> **その他**: `clone`, `panic`, `Box_new`, `unbox`

---

## 追加順序

ADR-004 により trait は v0 に入らない。これに基づいた追加順序:

### Phase 1: v0 初期（trait 不要）

| モジュール | 内容 | 依存 | 実装状況 |
|-----------|------|------|---------|
| `core/mem` | メモリ操作 | なし | 🔲 設計済み |
| `core/option` | `Option<T>` | なし | ✅ unwrap/unwrap_or/is_some/is_none 動作 |
| `core/result` | `Result<T, E>` | なし | ✅ unwrap/?演算子 動作 |

### Phase 2: v0 完成（trait 不要）

| モジュール | 内容 | 依存 | 実装状況 |
|-----------|------|------|---------|
| `collections/string` | String | Option | ✅ concat/slice/split/join/eq/is_empty 動作 |
| `collections/vec` | `Vec<T>` | Option | ✅ new/push/pop/get/set/len/sort 動作 |
| `io/fs` | ファイル操作 | Result, String | 🔲 設計済み |
| `io/clock` | 時刻 | なし | 🔲 設計済み |
| `io/random` | 乱数 | なし | 🔲 設計済み |

### Phase 3: v1 初期（trait 不要、P1・P2）

限定版 for と文字列補間は trait なしで導入可能:

| 機能 | 内容 | 依存 |
|------|------|------|
| 限定 `for` | `for i in 0..n`, `for x in values(v)` | 組み込み |
| 文字列補間 | `f"...{expr}..."` | 組み込み変換（プリミティブ型のみ）。カスタム型は P3（Display）が必要 |
| Vec 高階関数 | `any`, `find`, `map`, `filter` | 組み込み |

### Phase 4: v1 後期（trait 必要、P3〜P5）

| モジュール | 内容 | 必要な trait |
|-----------|------|-------------|
| `iter` | イテレータ | Iterator |
| `collections/hashmap` | HashMap | Eq, Hash |
| `fmt` | フォーマット | Display |
| `cmp` | 比較 | Ord, PartialOrd |

---

## v0 に入れないもの

- `iter`: trait 必須
- `HashMap`: Eq, Hash trait 必須
- `format!` 相当: Display trait 必須
- `async` / `Future`: 非同期ランタイム設計が必要
- `net`: async 設計前には入れない
- `sort`, `dedup`: Ord trait 必須

---

## 設計原則

### 1. 段階的な導入

trait がなくても基本機能は使える。trait 導入後に高度な機能を追加。

### 2. 明示的な API

LLM フレンドリのため、暗黙の変換や推論を減らす。
v0 ではメソッド構文なし。組み込み関数として提供。

```
// v0: 組み込み関数
let s = "hello"
let length = len(s)

// v1: メソッド構文
let length = s.len()
```

### 3. エラー処理は Result

panic は回復不能なバグ専用。通常のエラーは Result で表現。

---

## モジュール構成

```
std/
├── core/
│   ├── mem.ark       # メモリ操作
│   ├── option.ark    # Option<T>
│   └── result.ark    # Result<T, E>
│
├── collections/
│   ├── string.ark    # String
│   └── vec.ark       # Vec<T>
│
├── io/
│   ├── fs.ark        # ファイル操作
│   ├── clock.ark     # 時刻
│   └── random.ark    # 乱数
│
└── prelude.ark       # 自動インポート
```

### prelude

以下は自動的にインポートされる:
- `Option`, `Some`, `None`
- `Result`, `Ok`, `Err`
- `String`
- `Vec`
- `panic`, `len`, `println`, `print`
- `sqrt`, `abs`, `min`, `max`

---

## 関連

- ADR-004: trait 戦略
- `docs/stdlib/core.md`: core モジュール API
- `docs/stdlib/io.md`: I/O モジュール API
