# 値セマンティクス仕様

GC 採用下での copy/move 動作を厳密に定義する。

---

## 基本原則

**arukellt は「値型」と「参照型」を明確に区別する。**

- 値型: スタック上に置かれ、代入時にコピーされる
- 参照型: GC ヒープ上に置かれ、代入時に参照がコピーされる

---

## 型の分類

### 値型（Value Types）

| 型 | サイズ | コピーコスト |
|----|--------|-------------|
| `i32` | 4 bytes | O(1) |
| `i64` | 8 bytes | O(1) |
| `f32` | 4 bytes | O(1) |
| `f64` | 8 bytes | O(1) |
| `bool` | 1 byte (i32) | O(1) |
| `char` | 4 bytes (i32) | O(1) |
| `()` | 0 bytes | O(1) |
| tuple (全要素が値型) | Σ要素サイズ | O(n) |

### 参照型（Reference Types）

| 型 | ヒープ上のサイズ | コピーコスト |
|----|-----------------|-------------|
| `struct` | フィールドサイズ合計 | O(1) 参照コピー |
| `enum` | discriminant + 最大variant | O(1) 参照コピー |
| `String` | len + data | O(1) 参照コピー |
| `Vec[T]` | len + cap + data | O(1) 参照コピー |
| `[T]` (slice) | ref + len | O(1) 参照コピー |
| closure | func + env | O(1) 参照コピー |

---

## 代入の動作

### 値型の代入

```
let x: i32 = 42
let y = x      // x の値がコピーされる
y = y + 1      // y は 43、x は 42 のまま
```

**動作**: ビットコピー。両者は独立。

### 参照型の代入

```
let s1 = "hello"
let s2 = s1    // s1 の参照がコピーされる
// s1 と s2 は同じオブジェクトを指す
```

**動作**: 参照のコピー。オブジェクトは共有される。

```
// 図解
s1 ──┐
     ├──▶ [GC heap: "hello"]
s2 ──┘
```

**deep copy が必要な場合**: `clone` 関数を使用。

```
let s1 = "hello"
let s2 = clone(s1)  // 新しいオブジェクトが作られる（deep copy）
// s1 と s2 は別のオブジェクト
```

### ⚠️ 共有の暗黙性（最大の UX リスク）

**この言語の難しさは「所有権」ではなく「共有の暗黙性」にある。**

Rust や Go と異なり、参照型の代入は常に共有を意味する。
LLM と人間の両方が頻繁に間違えるパターン：

```
// ❌ 事故パターン: 意図しない共有
let a = Vec_new_i32()
push(a, 1)
let b = a        // b と a は同じ Vec を指す
push(b, 2)       // a も [1, 2] になる！

// ✅ 正しいパターン: 独立したコピーが必要な場合
let a = Vec_new_i32()
push(a, 1)
let b = clone(a)  // b は a の deep copy
push(b, 2)        // a は [1] のまま、b は [1, 2]
```

**設計上の立場**: これは仕様通りの動作であり、バグではない。
ただし UX リスクが高いため、以下で緩和する：

1. **診断**: 「代入後の可変操作」に対して W0001 warning を出す（→ diagnostics.md）
2. **Cookbook**: 共有 vs 独立のパターンを必ず掲載（→ cookbook.md）
3. **v1 検討**: `let b = copy a` のような明示的構文の導入

---

## 可変性との相互作用

### 値型の可変性

```
let mut x: i32 = 42
x = x + 1      // OK: x を変更
```

### 参照型の可変性

```
let mut v: Vec[i32] = Vec::new()
v.push(1)      // OK: v 経由でオブジェクトを変更
```

**注意**: `let v` (非 mut) でも、参照先のオブジェクト自体は変更可能。

```
let v: Vec[i32] = Vec::new()
v.push(1)      // OK: v の再代入はできないが、中身は変更可能
v = Vec::new() // NG: v への再代入はできない
```

これは意図的な設計。GC 環境では「参照の変更」と「オブジェクトの変更」を分離する。

---

## 深いコピー（clone）

参照型の深いコピーが必要な場合は明示的に `clone()` を呼ぶ。

```
let s1: String = String::from("hello")
let s2 = s1.clone()  // 新しい String オブジェクトを作成
// s1 と s2 は別のオブジェクト
```

**v0 での制限**: `clone()` は trait が必要なため、組み込み型のみ提供。

| 型 | clone() | 動作 |
|----|---------|------|
| `String` | ✅ 提供 | 新しい String を作成（内容をコピー） |
| `Vec<T>` | ✅ 提供 | **deep copy**: Vec 自体 + 全要素を再帰的に clone |
| ユーザー struct | ❌ v1 以降 | — |

**重要: `clone` は常に deep copy（再帰的複製）**

```
// Vec<String> の clone
let a = Vec_new_String()
push(a, "hello")
push(a, "world")

let b = clone(a)
// b は新しい Vec、中身も新しい String
// a と b は完全に独立
```

```
// 図解: clone(a) の結果
a ──▶ [Vec: "hello", "world"]  ← GC heap 上のオブジェクト A
b ──▶ [Vec: "hello", "world"]  ← GC heap 上のオブジェクト B（完全に別物）
```

**コストモデル**:

| 操作 | コスト |
|------|--------|
| `clone(x: i32)` | O(1) — ビットコピー |
| `clone(s: String)` | O(n) — 文字列長に比例 |
| `clone(v: Vec<T>)` | O(n × clone(T)) — 要素数 × 要素の clone コスト |

**注意**: deep clone はコストが高い。不要な clone を避けること。
参照共有で十分な場合は `let b = a` を使う（→ 共有の暗黙性に注意）。

---

## 関数引数の受け渡し

### 値型

```
fn double(x: i32) -> i32 {
    x * 2
}

let a = 42
let b = double(a)  // a はコピーされて渡される
// a は 42 のまま
```

### 参照型

```
fn append(v: Vec[i32], x: i32) {
    v.push(x)  // v 経由でオブジェクトを変更
}

let list = Vec::new()
append(list, 1)
// list は [1] になっている（同じオブジェクト）
```

---

## 戻り値

```
fn make_string() -> String {
    String::from("hello")
}

let s = make_string()  // GC heap 上のオブジェクトへの参照が返る
```

---

## コストモデル

### O(1) 操作

- 値型の代入
- 参照型の代入（参照コピー）
- フィールドアクセス
- 配列インデックス（bounds check あり）

### O(n) 操作

- `String.clone()`: 文字列長に比例
- `Vec[T].clone()`: 要素数に比例
- 値型 tuple の代入: 要素数に比例

### GC コスト

- オブジェクト生成: O(1) 平均
- GC 発生時: O(live objects) — 予測困難

---

## move の扱い

**v0 では move を導入しない。**

理由:

- GC 環境では move の利点が薄い
- 複雑な所有権規則を避ける（LLM フレンドリ）

結果:

- 使用後も変数は有効
- 「二重使用」エラーは発生しない

```
let s = String::from("hello")
let s2 = s   // 参照コピー
let s3 = s   // OK: s はまだ有効
```

---

## 参照の同一性

`===` 演算子（v1 以降）で参照の同一性を確認可能にする予定。

v0 では同一性チェックは提供しない。

---

## 関連

- `docs/language/memory-model.md`: Wasm GC での型表現
- `docs/design/gc-mono-tradeoff.md`: サイズ/性能トレードオフ
- `docs/design/reference-control.md`: 参照過多への制御
