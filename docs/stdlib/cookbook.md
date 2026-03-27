# Core API Cookbook

> **Current-first**: 実装の現在地は [../current-state.md](../current-state.md) を参照してください。

このページは、現行実装でそのまま使いやすい書き方だけを残した cookbook です。
古い capability API や未確認 helper は削っています。

## 基本方針

1. 関数呼び出し形式を基準にする
2. `get` / `pop` の戻り値は `Option<T>` として扱う
3. `Result<T, String>` は `match` または `?` で処理する
4. v1 機能があっても、まずは Prelude ベースの書き方を優先する

## Vec

### 作成と追加

```ark
let v: Vec<i32> = Vec_new_i32()
push(v, 10)
push(v, 20)
```

### 取得

```ark
let x = get(v, 0)
match x {
    Some(value) => println(i32_to_string(value)),
    None => println(String_from("out of bounds")),
}
```

### 安全でない取得

```ark
let x: i32 = get_unchecked(v, 0)
println(i32_to_string(x))
```

### map / filter / fold

```ark
fn double(x: i32) -> i32 { x * 2 }
fn is_even(x: i32) -> bool { x % 2 == 0 }
fn sum(acc: i32, x: i32) -> i32 { acc + x }

let mapped = map_i32_i32(v, double)
let filtered = filter_i32(mapped, is_even)
let total = fold_i32_i32(filtered, 0, sum)
println(i32_to_string(total))
```

### 追加 helper

```ark
sort_i32(v)
let found = find_i32(v, is_even)
let has_20 = contains_i32(v, 20)
reverse_i32(v)
```

## String

### 作成と連結

```ark
let s1 = String_from("hello")
let s2 = String_from(" world")
let s3 = concat(s1, s2)
println(s3)
```

### slice / split / join

```ark
let sub = slice(s3, 0, 5)
let parts = split(String_from("a,b,c"), String_from(","))
let joined = join(parts, String_from("-"))
println(sub)
println(joined)
```

### 変換

```ark
println(i32_to_string(42))
println(i64_to_string(clock_now()))
println(bool_to_string(true))
```

## Option

### 基本

```ark
let x: Option<i32> = Some(21)
if is_some(x) {
    println(i32_to_string(unwrap(x)))
}
```

### unwrap_or

```ark
let y: i32 = unwrap_or(get(v, 100), 0)
println(i32_to_string(y))
```

### map_option_i32_i32

```ark
fn double(x: i32) -> i32 { x * 2 }
let x: Option<i32> = Some(21)
let y: Option<i32> = map_option_i32_i32(x, double)
```

## Result

### match で処理

```ark
let r = parse_i32(String_from("42"))
match r {
    Ok(n) => println(i32_to_string(n)),
    Err(e) => println(e),
}
```

### `?` で伝播

```ark
fn parse_positive(s: String) -> Result<i32, String> {
    let n = parse_i32(s)?
    if n < 0 {
        return Err(String_from("negative"))
    }
    Ok(n)
}
```

## Filesystem

```ark
fn main() {
    let r = fs_read_file(String_from("input.txt"))
    match r {
        Ok(content) => {
            print(content)
            let _ = fs_write_file(String_from("output.txt"), content)
        }
        Err(e) => println(e),
    }
}
```

## Clock / Random

```ark
println(i64_to_string(clock_now()))
println(i32_to_string(random_i32()))
```

## v1 feature note

このブランチではメソッド構文や拡張構文も入っていますが、
共通で通しやすいサンプルとしてこの cookbook では関数呼び出し形式を優先しています。

## 関連

- [core.md](core.md)
- [io.md](io.md)
- [../quickstart.md](../quickstart.md)
- [../current-state.md](../current-state.md)
