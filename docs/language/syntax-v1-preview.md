# v1 以降の構文（プレビュー）

このファイルは v1 で追加予定の構文を記録する。
**v0 では使用不可。**

---

## メソッド構文

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

---

## 構造体パターン

```
match point {
    Point { x, y } => x + y,
}
```

---

## ガード

```
match x {
    n if n > 0 => "positive",
    n if n < 0 => "negative",
    _ => "zero",
}
```

---

## or-pattern

```
match x {
    1 | 2 | 3 => "small",
    _ => "large",
}
```

---

## match でのタプルパターン

```
match pair {
    (0, y) => y,
    (x, 0) => x,
    (x, y) => x + y,
}
```

---

## for ループ

trait 導入後:

```
for item in items {
    process(item)
}
```

---

## 演算子オーバーロード

trait 導入後:

```
impl Add for Point {
    fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
}
```

---

## ? 演算子のエラー型自動変換

trait 導入後、`From<E>` による自動変換:

```
fn read_and_parse(path: String) -> Result<i64, AppError> {
    let content = read_file(path)?  // IoError -> AppError 自動変換
    let n = parse_int(content)?     // ParseError -> AppError 自動変換
    Ok(n)
}
```
