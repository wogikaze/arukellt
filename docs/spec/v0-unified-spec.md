# Arukellt v0 統合仕様書

本文書は、arukellt 言語 v0 の完全仕様を一箇所に統合したものである。

---

## 1. 言語概要

Arukellt は Wasm-first、LLM-friendly を目指す静的型付け言語。

### 設計原則

1. **Wasm が正**: Wasm 意味論が唯一の動作定義
2. **簡潔さ優先**: 性能より理解しやすさ
3. **GC 採用**: 所有権/借用の複雑さを回避
4. **制限付き機能**: 必要最小限の言語機能

### v0 で提供するもの

- 基本的な型システム（プリミティブ、struct、enum、tuple）
- 制限付き generics（`Vec<T>`、`Option<T>`、`Result<T, E>`）
- 高階関数と closure
- パターンマッチ（リテラル、enum variant、ワイルドカード、変数束縛）
- WASI p1 サポート

### v0 で提供しないもの

- trait / interface
- impl / メソッド構文
- iterator / for ループ
- 演算子オーバーロード
- マクロ
- async/await

---

## 2. 型システム

### 2.1 プリミティブ型

| 型 | サイズ | 説明 |
|----|--------|------|
| `i32` | 32bit | 符号付き整数 |
| `i64` | 64bit | 符号付き整数 |
| `f32` | 32bit | 浮動小数点 |
| `f64` | 64bit | 浮動小数点 |
| `bool` | 1bit | true/false |
| `char` | 32bit | Unicode scalar value |
| `()` | 0bit | Unit 型 |

### 2.2 複合型

**Struct**
```
struct Point {
    x: f64,
    y: f64,
}
```

**Enum**
```
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}
```

**Tuple**
```
let pair: (i32, String) = (42, "hello")
```

### 2.3 組み込み generics

制限付き monomorphization を採用。

| 型 | 説明 | 制限 |
|----|------|------|
| `Vec<T>` | 可変長配列 | T はネスト不可 |
| `Option<T>` | 省略可能値 | T はネスト不可 |
| `Result<T, E>` | エラー処理 | T, E はネスト不可 |

**許可される例**:
```
Vec<i32>
Vec<String>
Option<Point>
Result<i32, String>
```

**禁止される例**:
```
Vec<Vec<i32>>       // ネスト禁止
Vec<Option<i32>>    // ネスト禁止
```

### 2.4 関数型

```
fn(i32) -> i32           // 引数 i32、戻り値 i32
fn(String, i32) -> bool  // 複数引数
fn() -> ()               // 引数なし、戻り値なし
```

---

## 3. メモリモデル

### 3.1 GC 採用

Wasm GC 提案を使用。手動メモリ管理不要。

### 3.2 値セマンティクス

| 型カテゴリ | コピー動作 | 例 |
|-----------|-----------|-----|
| 値型 | ビット単位コピー | i32, f64, bool, tuple |
| 参照型 | 参照コピー（オブジェクト共有） | String, Vec, struct, enum, [T] |

```
let a: i32 = 42
let b = a        // 値コピー（独立）

let s1 = "hello"
let s2 = s1      // 参照コピー（同じオブジェクト）
```

### 3.3 clone

明示的な深いコピーには `clone` 関数を使用:

```
let s1 = "hello"
let s2 = clone(s1)  // 新しいオブジェクト
```

### 3.4 move なし

v0 では move セマンティクスを導入しない。すべての代入はコピー。

---

## 4. 構文

### 4.1 変数宣言

```
let x = 42              // 不変（再束縛不可）
let mut y = 0           // 可変（再束縛可能）
```

### 4.2 関数定義

```
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn greet(name: String) {
    print(concat("Hello, ", name))
}
```

### 4.3 構造体

v0 ではメソッド構文なし。モジュールレベル関数として定義:

```
struct Point {
    x: f64,
    y: f64,
}

fn point_new(x: f64, y: f64) -> Point {
    Point { x: x, y: y }
}

fn point_distance(p: Point, other: Point) -> f64 {
    let dx = p.x - other.x
    let dy = p.y - other.y
    sqrt(dx * dx + dy * dy)
}
```

### 4.4 列挙型

```
enum Option<T> {
    Some(T),
    None,
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

### 4.5 パターンマッチ

v0 でサポートするパターン:
- リテラル
- enum variant
- ワイルドカード `_`
- 変数束縛

```
match value {
    Some(x) => x * 2,
    None => 0,
}

match shape {
    Shape::Circle(r) => 3.14 * r * r,
    Shape::Rectangle(w, h) => w * h,
}
```

### 4.6 制御構文

```
// if 式
let result = if condition { a } else { b }

// while ループ
let mut i = 0
while i < 10 {
    i = i + 1
}

// loop ループ
loop {
    if done { break }
}
```

### 4.7 Closure

```
let add = |a, b| a + b
let result = add(1, 2)

// キャプチャ
let n = 10
let add_n = |x| x + n
```

---

## 5. 標準ライブラリ

### 5.1 v0 で提供する module

| Module | 内容 |
|--------|------|
| `core` | Option, Result, panic |
| `string` | String 操作 |
| `vec` | Vec 操作 |
| `io` | WASI I/O |
| `mem` | メモリユーティリティ |

### 5.2 String API

v0 ではメソッド構文なし。組み込み関数として提供:

```
fn string_new() -> String
fn len(s: String) -> i32
fn is_empty(s: String) -> bool
fn concat(a: String, b: String) -> String
fn string_slice(s: String, start: i32, end: i32) -> String
fn clone(s: String) -> String
```

String は不変（immutable）。変更には新しい String を作成。

### 5.3 Vec API

```
fn vec_new<T>() -> Vec<T>
fn len<T>(v: Vec<T>) -> i32
fn is_empty<T>(v: Vec<T>) -> bool
fn vec_push<T>(v: Vec<T>, val: T)
fn vec_pop<T>(v: Vec<T>) -> Option<T>
fn vec_get<T>(v: Vec<T>, idx: i32) -> Option<T>
fn vec_set<T>(v: Vec<T>, idx: i32, val: T)
fn clone<T>(v: Vec<T>) -> Vec<T>
```

型特化関数（trait がないため）:
```
fn vec_i32_map(v: Vec<i32>, f: fn(i32) -> i32) -> Vec<i32>
fn vec_i32_filter(v: Vec<i32>, f: fn(i32) -> bool) -> Vec<i32>
fn vec_i32_sort(v: Vec<i32>)
```

### 5.4 Option API

```
fn is_some<T>(opt: Option<T>) -> bool
fn is_none<T>(opt: Option<T>) -> bool
fn unwrap<T>(opt: Option<T>) -> T
fn unwrap_or<T>(opt: Option<T>, default: T) -> T
fn option_map_i32(opt: Option<i32>, f: fn(i32) -> i32) -> Option<i32>
```

### 5.5 Result API

```
fn is_ok<T, E>(res: Result<T, E>) -> bool
fn is_err<T, E>(res: Result<T, E>) -> bool
fn unwrap<T, E>(res: Result<T, E>) -> T
fn unwrap_err<T, E>(res: Result<T, E>) -> E
fn result_map_i32(res: Result<i32, E>, f: fn(i32) -> i32) -> Result<i32, E>
```

### 5.6 I/O API

Capability-based 設計:

```
fn main(caps: Caps) {
    // stdin/stdout/stderr
    stdout_write(caps, "Hello\n")
    let line = stdin_read_line(caps)
    
    // ファイルシステム（DirCap + RelPath）
    let dir = preopened_dir(caps, ".")
    let content = read_file(dir, "data.txt")
}
```

---

## 6. Wasm 生成

### 6.1 使用する Wasm 機能

**Layer 1（必須）**:
- Wasm GC: struct, array, i31ref
- Multi-value returns
- Reference types
- WASI p1

**Layer 2（オプション）**:
- Tail call
- Exception handling

### 6.2 型マッピング

| arukellt | Wasm GC |
|----------|---------|
| i32 | i32 |
| i64 | i64 |
| f32 | f32 |
| f64 | f64 |
| bool | i32 (0/1) |
| char | i32 |
| String | (ref $string) where $string = (struct (field (ref $array_u8))) |
| Vec<T> | (ref (array T)) |
| struct | (ref (struct ...)) |
| enum | (ref (struct (field i32) (field (ref $data)))) |

### 6.3 ABI

3 層構造:

| Layer | 用途 | 規約 |
|-------|------|------|
| 1 | 内部 | 非公開、変更可能 |
| 2 | Wasm | GC 参照で渡す |
| 3 | native | C ABI（値コピー） |

---

## 7. コンパイラフェーズ

```
Source (.ark)
    │
    ▼
┌─────────┐
│  Lexer  │
└────┬────┘
     │ Token stream
     ▼
┌─────────┐
│ Parser  │
└────┬────┘
     │ AST
     ▼
┌─────────────────┐
│ Name Resolution │
└────────┬────────┘
         │ Resolved AST
         ▼
┌─────────────────┐
│  Type Checker   │
└────────┬────────┘
         │ Typed AST
         ▼
┌─────────────────┐
│   MIR Lower     │
└────────┬────────┘
         │ MIR
         ▼
┌─────────────────┐
│  Wasm Emitter   │
└────────┬────────┘
         │
         ▼
    Output (.wasm)
```

---

## 8. トレードオフと制限

### 8.1 GC + Mono の緊張

| 選択 | 効果 | 副作用 |
|------|------|--------|
| GC | バイナリサイズ減 | すべてヒープに逃げやすい |
| Mono | 実行速度 | バイナリサイズ増 |

解決策: 値型は特化、参照型は統一表現。

### 8.2 trait なし

抽象化は型ごとの関数で代替:
- `vec_i32_map`, `vec_string_map` など
- v1 で trait 導入後に移行

### 8.3 GC ⇔ C 境界

直接変換なし。linear memory 経由でデータ交換。

---

## 9. 関連文書

| 文書 | 内容 |
|------|------|
| ADR-002 | GC 採用の決定 |
| ADR-003 | generics 戦略 |
| ADR-004 | trait 戦略 |
| ADR-005 | LLVM の役割 |
| ADR-006 | ABI 方針 |
| `docs/design/value-semantics.md` | 値セマンティクス詳細 |
| `docs/design/gc-mono-tradeoff.md` | サイズ/性能トレードオフ |
| `docs/design/gc-c-abi-bridge.md` | FFI 境界設計 |
| `docs/design/trait-less-abstraction.md` | 抽象化戦略 |
| `docs/design/reference-control.md` | 参照過多の制御 |

---

## 10. 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2025-01-XX | 初版作成 |
| 2026-03-24 | v0 canonical surface に統一（generics `<T>`、メソッドなし）|
