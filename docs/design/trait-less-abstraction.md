# trait なし環境での抽象化戦略

ADR-004 により v0 では trait を導入しない。その制約下での抽象化方法を定義する。

---

## 問題

trait なしでは以下ができない:
- 型をまたぐ共通インターフェース
- Iterator / map / filter
- 演算子オーバーロード
- ユーザー定義の再利用可能な抽象

**v0 は「言語」というより「型付きスクリプト環境」に近くなる。**

これを許容した上で、最低限の抽象化手段を提供する。

---

## 提供する抽象化手段

### 1. 高階関数

**許可する。** closure を引数/戻り値として渡せる。

```
fn map_i32(arr: [i32], f: fn(i32) -> i32) -> Vec[i32] {
    let mut result = Vec::new()
    let mut i = 0
    while i < arr.len() {
        result.push(f(arr[i]))
        i = i + 1
    }
    result
}

// 使用
let doubled = map_i32([1, 2, 3], |x| x * 2)
```

**制限**: 型ごとに関数を書く必要がある（trait がないため）。

### 2. Closure

**許可する。** 環境をキャプチャできる。

```
fn make_adder(n: i32) -> fn(i32) -> i32 {
    |x| x + n  // n をキャプチャ
}

let add5 = make_adder(5)
let result = add5(10)  // 15
```

**キャプチャの動作**:
- 値型: 値コピー
- 参照型: 参照コピー（オブジェクト共有）

### 3. 組み込み操作関数

trait なしでも使える組み込み関数を提供:

| カテゴリ | 関数 | 説明 |
|----------|------|------|
| **比較** | `i32.eq(a, b)` | 等値比較 |
| | `i32.lt(a, b)` | 小なり |
| | `String.eq(a, b)` | 文字列等値 |
| **Vec 操作** | `Vec[i32].map(v, f)` | 各要素に f を適用 |
| | `Vec[i32].filter(v, f)` | 条件を満たす要素を抽出 |
| | `Vec[i32].fold(v, init, f)` | 畳み込み |
| **String 操作** | `String.split(s, sep)` | 分割 |
| | `String.join(arr, sep)` | 結合 |

**実装方法**: 型ごとに専用関数を標準ライブラリに用意。

```
// 標準ライブラリ提供（内部実装）
pub fn Vec_i32_map(v: Vec[i32], f: fn(i32) -> i32) -> Vec[i32] { ... }
pub fn Vec_i64_map(v: Vec[i64], f: fn(i64) -> i64) -> Vec[i64] { ... }
pub fn Vec_String_map(v: Vec[String], f: fn(String) -> String) -> Vec[String] { ... }
```

### 4. パターンマッチによる分岐

enum + match で型安全な分岐を提供:

```
enum Shape {
    Circle(f64),           // radius
    Rectangle(f64, f64),   // width, height
}

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rectangle(w, h) => w * h,
    }
}
```

これは trait なしでも動作する。

---

## 提供しない抽象化

### 1. 汎用 map/filter

```
// これは書けない（trait が必要）
fn map<T, U>(arr: [T], f: fn(T) -> U) -> Vec<U>
```

**代替**: 型ごとに `Vec_i32_map`, `Vec_String_map` 等を使用。

### 2. 演算子オーバーロード

```
// これは書けない
struct Point { x: f64, y: f64 }
// Point + Point は定義できない
```

**代替**: 明示的なメソッド呼び出し。

```
fn Point::add(self, other: Point) -> Point {
    Point { x: self.x + other.x, y: self.y + other.y }
}

let p3 = p1.add(p2)  // p1 + p2 の代わり
```

### 3. 共通インターフェース

```
// これは書けない
trait Printable {
    fn to_string(self) -> String
}
```

**代替**: 各型に `to_string()` メソッドを個別に定義。呼び出し側で型を知っている必要がある。

---

## コード再利用のパターン

### パターン 1: 型特化関数群

```
// 数値の範囲チェック
fn i32_in_range(x: i32, min: i32, max: i32) -> bool {
    x >= min && x <= max
}

fn f64_in_range(x: f64, min: f64, max: f64) -> bool {
    x >= min && x <= max
}
```

**トレードオフ**: コードの重複が発生するが、型安全性は保たれる。

### パターン 2: 構造体 + メソッド

```
struct Counter {
    value: i32,
}

impl Counter {
    fn new() -> Counter { Counter { value: 0 } }
    fn increment(self) { self.value = self.value + 1 }
    fn get(self) -> i32 { self.value }
}
```

### パターン 3: 高階関数による抽象化

```
fn repeat(n: i32, action: fn()) {
    let mut i = 0
    while i < n {
        action()
        i = i + 1
    }
}

repeat(5, || print("hello"))
```

---

## v0 での現実的な期待値

| やりたいこと | v0 での方法 | 難易度 |
|-------------|------------|--------|
| リストの変換 | `Vec_T_map` 関数 | 低（組み込み） |
| フィルタリング | `Vec_T_filter` 関数 | 低（組み込み） |
| ソート | `Vec_i32_sort` 等 | 低（組み込み） |
| カスタム型のソート | ❌ v1 以降 | - |
| 複数型をまとめて扱う | パターンマッチ | 中 |
| プラグイン的拡張 | ❌ v1 以降 | - |

---

## v1 への移行パス

v1 で trait を導入した際:
- 組み込み関数 → trait メソッドに移行
- 既存コードは動作を維持（後方互換）
- 新しい抽象化パターンが利用可能に

---

## 関連

- ADR-004: trait 戦略
- `docs/stdlib/README.md`: 標準ライブラリの提供関数
- `docs/process/v0-scope.md`: v0 スコープ
