# Arukellt Quickstart

現在の実装でまず動く書き方だけに絞ったガイドです。
詳細な実装状況は [current-state.md](current-state.md) を参照してください。

## Hello World

```ark
use std::host::stdio

fn main() {
    stdio::print("Hello, world!")
}
```

```bash
arukellt run hello.ark
```

## Component Build

WIT / Component Model output is available on the T3 target.

```bash
arukellt compile --target wasm32-wasi-p2 --emit wit hello.ark
arukellt compile --target wasm32-wasi-p2 --emit component hello.ark
```

To bind host imports from external WIT, pass one or more `--wit` files:

```bash
arukellt compile --target wasm32-wasi-p2 --emit component app.ark --wit host.wit
```

`--emit all` produces both `app.wasm` and `app.component.wasm`.

## 基本型

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let n: i32 = 42
let big: i64 = 1000000
let f: f64 = 3.14
let b: bool = true
let s: String = String_from("hello")
```

## Vec

```ark
use std::host::stdio

fn main() {
    let v: Vec<i32> = Vec_new_i32()
    push(v, 10)
    push(v, 20)

    let first: Option<i32> = get(v, 0)
    match first {
        Some(x) => stdio::println(to_string(x)),
        None => stdio::println(String_from("empty")),
    }

    stdio::println(to_string(len(v)))
}
```

`get(v, i)` は `Option<T>` を返します。要素が必ずあると分かっている場面だけ `get_unchecked(v, i)` を使ってください。

## String

```ark
use std::host::stdio

fn main() {
    let s1 = String_from("hello")
    let s2 = String_from(" world")
    let s3 = concat(s1, s2)
    stdio::println(s3)

    let sub = slice(s3, 0, 5)
    stdio::println(sub)
}
```

## Option / Result

```ark
use std::host::stdio

fn parse_positive(s: String) -> Result<i32, String> {
    let n = parse_i32(s)?
    if n < 0 {
        return Err(String_from("negative value"))
    }
    Ok(n)
}

fn main() {
    match parse_positive(String_from("42")) {
        Ok(n) => stdio::println(to_string(n)),
        Err(e) => stdio::println(e),
    }
}
```

## Filesystem I/O

現行実装では host access を `std::host::*` から明示 import します。

```ark
use std::host::fs
use std::host::stdio

fn main() {
    let r = fs::read_to_string("input.txt")
    match r {
        Ok(content) => stdio::print(content),
        Err(e) => stdio::println(e),
    }
}
```

- `fs::read_to_string(path: String) -> Result<String, FsError>`
- `fs::write_string(path: String, content: String) -> Result<(), FsError>`

On error, match on `FsError` variants: `NotFound(String)`, `PermissionDenied(String)`,
`Utf8Error`, `IoError(String)`. Use `fs::fs_error_message(err)` for a plain string message.

## 文字列化

`to_string(x)` を基準に使うのが一番安定です。`i32_to_string` などの型別 helper は互換用として残っています。

## Clock / Random

```ark
use std::host::clock
use std::host::random as host_random
use std::host::stdio

fn main() {
    stdio::println(to_string(clock::monotonic_now()))
    stdio::println(to_string(host_random::random_i32()))
}
```

## v1 features

このブランチでは v1 系の一部機能も使えますが、まずは上の関数呼び出しスタイルを基準にするのが安全です。
必要なら以下を参照してください。

- [language/syntax.md](language/syntax.md)
- [language/syntax-v1-preview.md](language/syntax-v1-preview.md)
- [current-state.md](current-state.md)
