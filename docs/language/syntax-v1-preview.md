# v1 以降の構文（プレビュー）

このファイルは v1 で追加予定の構文を記録する。
**v0 では使用不可。**

優先度は parser.rs → parser.ark 翻訳の実測結果に基づく。
詳細: `docs/process/parser-ark-evaluation.md`

**注**: `break` / `continue` は v0 に含まれている。v1 項目ではない。

---

## P1: for ループ（限定版、trait 不要）

trait ベースの Iterator を待たずに導入可能。解決規則を増やさない。

```
// 範囲ベース
for i in 0..len(v) {
    let item = get(v, i)
}

// Vec 走査（組み込み）
for item in values(v) {
    process(item)
}
```

**設計ポイント**: `0..n` は組み込みの範囲式。`values(v)` は Vec 専用の組み込みイテレータ。trait 不要。

---

## P2: 文字列補間

```
let name = "world"
let msg = f"Hello, {name}!"
let result = f"Expected {expected}, got {actual}"
```

concat ネスト `concat(a, concat(b, c))` の完全解消。diagnostics 実装の品質に直結。

---

## P3: trait / iterator

### trait 定義

```
trait Display {
    fn display(self) -> String
}

trait Iterator<T> {
    fn next(self) -> Option<T>
}
```

### trait 実装

```
impl Display for Point {
    fn display(self) -> String {
        f"({self.x}, {self.y})"
    }
}
```

### trait ベース for ループ（P1 の拡張）

```
for item in items {
    process(item)
}
```

P1 の限定 for を一般化。任意の Iterator 実装に対して動作。

### ? 演算子のエラー型自動変換

`From<E>` trait による自動変換:

```
fn read_and_parse(path: String) -> Result<i64, AppError> {
    let content = read_file(path)?  // IoError -> AppError 自動変換
    let n = parse_int(content)?     // ParseError -> AppError 自動変換
    Ok(n)
}
```

---

## P4: メソッド構文

```
impl Point {
    fn new(x: f64, y: f64) -> Point {
        Point { x: x, y: y }
    }
    
    fn distance(self, other: Point) -> f64 {
        let dx = self.x - other.x
        let dy = self.y - other.y
        sqrt(dx * dx + dy * dy)
    }
}

let d = p.distance(other)
```

**注**: 読みやすさ向上だが本質改善ではない。`point_distance(p, other)` は `p.distance(other)` と同等に機能する。

---

## P5: 演算子オーバーロード

trait 導入後:

```
impl Add for Point {
    fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
}
```

---

## 優先度未定: パターン拡張

### 構造体パターン

```
match point {
    Point { x, y } => x + y,
}
```

### ガード

```
match x {
    n if n > 0 => "positive",
    n if n < 0 => "negative",
    _ => "zero",
}
```

### or-pattern

```
match x {
    1 | 2 | 3 => "small",
    _ => "large",
}
```

### match でのタプルパターン

```
match pair {
    (0, y) => y,
    (x, 0) => x,
    (x, y) => x + y,
}
```
