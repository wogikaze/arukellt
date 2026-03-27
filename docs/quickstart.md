# Arukellt Quickstart

現在の実装でまず動く書き方だけに絞ったガイドです。
詳細な実装状況は [current-state.md](current-state.md) を参照してください。

## Hello World

```ark
fn main() {
    print("Hello, world!")
}
```

```bash
arukellt run hello.ark
```

## 基本型

```ark
let n: i32 = 42
let big: i64 = 1000000
let f: f64 = 3.14
let b: bool = true
let s: String = String_from("hello")
```

## Vec

```ark
fn main() {
    let v: Vec<i32> = Vec_new_i32()
    push(v, 10)
    push(v, 20)

    let first: Option<i32> = get(v, 0)
    match first {
        Some(x) => println(i32_to_string(x)),
        None => println(String_from("empty")),
    }

    println(i32_to_string(len(v)))
}
```

`get(v, i)` は `Option<T>` を返します。要素が必ずあると分かっている場面だけ `get_unchecked(v, i)` を使ってください。

## String

```ark
fn main() {
    let s1 = String_from("hello")
    let s2 = String_from(" world")
    let s3 = concat(s1, s2)
    println(s3)

    let sub = slice(s3, 0, 5)
    println(sub)
}
```

## Option / Result

```ark
fn parse_positive(s: String) -> Result<i32, String> {
    let n = parse_i32(s)?
    if n < 0 {
        return Err(String_from("negative value"))
    }
    Ok(n)
}

fn main() {
    match parse_positive(String_from("42")) {
        Ok(n) => println(i32_to_string(n)),
        Err(e) => println(e),
    }
}
```

## Filesystem I/O

現行実装では capability 引数ではなく、直接 wrapper を呼びます。

```ark
fn main() {
    let r = fs_read_file(String_from("input.txt"))
    match r {
        Ok(content) => print(content),
        Err(e) => println(e),
    }
}
```

- `fs_read_file(path: String) -> Result<String, String>`
- `fs_write_file(path: String, content: String) -> Result<(), String>`

## Clock / Random

```ark
fn main() {
    println(i64_to_string(clock_now()))
    println(i32_to_string(random_i32()))
}
```

## v1 features

このブランチでは v1 系の一部機能も使えますが、まずは上の関数呼び出しスタイルを基準にするのが安全です。
必要なら以下を参照してください。

- [language/syntax.md](language/syntax.md)
- [language/syntax-v1-preview.md](language/syntax-v1-preview.md)
- [current-state.md](current-state.md)
