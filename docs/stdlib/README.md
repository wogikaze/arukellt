# 標準ライブラリ設計方針

---

## 追加順序

ADR-004 により trait は v0 に入らない。これに基づいた追加順序:

### Phase 1: v0 初期（trait 不要）

| モジュール | 内容 | 依存 |
|-----------|------|------|
| `core/mem` | メモリ操作 | なし |
| `core/option` | Option[T] | なし |
| `core/result` | Result[T, E] | なし |

### Phase 2: v0 完成（trait 不要）

| モジュール | 内容 | 依存 |
|-----------|------|------|
| `collections/string` | String / str | Option |
| `collections/vec` | Vec[T] | Option |
| `io/fs` | ファイル操作 | Result, String |
| `io/clock` | 時刻 | なし |
| `io/random` | 乱数 | なし |

### Phase 3: v1 以降（trait 必要）

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

```
// OK: 明示的
let s = String::from("hello")
let len = s.len()

// NG: 暗黙の変換（trait 必要）
let s: String = "hello"  // v0 では使えない
```

### 3. エラー処理は Result

panic は回復不能なバグ専用。通常のエラーは Result で表現。

---

## モジュール構成

```
std/
├── core/
│   ├── mem.ark       # メモリ操作
│   ├── option.ark    # Option[T]
│   └── result.ark    # Result[T, E]
│
├── collections/
│   ├── string.ark    # String / str
│   └── vec.ark       # Vec[T]
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
- `panic`

---

## 関連

- ADR-004: trait 戦略
- `docs/stdlib/core.md`: core モジュール API
- `docs/stdlib/io.md`: I/O モジュール API
