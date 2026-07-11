# Arukellt Quickstart

現在の実装で動く core examples と、未完成機能の provisional preview を
明確に分けたガイドです。
詳細な実装状況は [current-state.md](current-state.md) を参照してください。

> 各例は fixture registry へ登録され、CI のコンパイル・実行対象です。
> fixture は `tests/fixtures/quickstart/` にあります。
> 最新の harness snapshot で全例が pass している保証は [current-state.md](current-state.md) を参照してください。

## Hello World

<!-- fixture: quickstart/hello.ark -->
```ark
use std::host::stdio

fn main() {
    stdio::print("Hello, world!")
}
```

```bash
arukellt run hello.ark
```

## Provisional Preview: Component Build

> ⚠️ **Provisional / 103件失敗中**: Component Model output は `wasm32-gc` で一部利用可能ですが、
> 現在 `verify component-interop` が 103/103 失敗中です。
> Library exports は s2 wasm 条件付き、WIT coverage は partial です。
> 詳細は [current-state.md](current-state.md) と
> [data/release-guarantees.md](data/release-guarantees.md) の `emit_component` 行を参照してください。

```bash
arukellt compile --target wasm32-gc --emit wit hello.ark
arukellt compile --target wasm32-gc --emit component hello.ark
```

Cross-language interop walkthroughs (Ark ↔ Rust ↔ JS, compose / WIT import):
see [`../examples/README.md`](../examples/README.md).

To bind host imports from external WIT, pass one or more `--wit` files:

```bash
arukellt compile --target wasm32-gc --emit component app.ark --wit host.wit
```

`--emit all` produces both `app.wasm` and `app.component.wasm`.

## 基本型

<!-- fixture: quickstart/basic_types.ark -->
```ark
use std::host::stdio

fn main() {
    let n: i32 = 42
    let b: bool = true
    let s: String = String_from("hello")
    stdio::println(to_string(n))
    stdio::println(to_string(b))
    stdio::println(s)
}
```

> `i64` と `f64` は現行 selfhost で `to_string` 経由の出力に一部制約があります。
> 詳細は [current-state.md](current-state.md) を参照してください。

## Vec

> **API note**: `Vec_new_i32()` は現行 bootstrap で動く provisional
> constructorです。`Vec::new<i32>()`は未実装のplanned APIであり、current
> replacementではありません。

<!-- fixture: quickstart/vec_basic.ark -->
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

`concat()`のdeprecated prelude wrapperではなく、現在利用可能な
`std::text::concat`を明示importします。planned method syntaxはcore pathで
案内しません。

<!-- fixture: quickstart/string_basic.ark -->
```ark
use std::host::stdio
use std::text

fn main() {
    let s1 = String_from("hello")
    let s2 = String_from(" world")
    let s3 = text::concat(s1, s2)
    stdio::println(s3)

    let sub = slice(s3, 0, 5)
    stdio::println(sub)
}
```

## Option / Result

<!-- fixture: quickstart/option_result.ark -->
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

<!-- fixture: quickstart/fs_read.ark -->
```ark
use std::host::fs
use std::host::stdio

fn main() {
    let r = fs::read_to_string("tests/fixtures/quickstart/fs_read_input.txt")
    match r {
        Ok(content) => stdio::print(content),
        Err(e) => stdio::println(e),
    }
}
```

現行の Filesystem API signature（manifest-backed）:

- `fs::read_to_string(path: String) -> Result<String, String>`
- `fs::write_string(path: String, content: String) -> Result<(), String>`

エラーは `String` で返ります。`FsError` enum は同じ module に存在しますが、
現行の `read_to_string` / `write_string` は typed error ではなく `String` を返します。
`FsError` は `read_dir` / `metadata` 等の将来の typed fs API で使われます。

> API signature の正本は [stdlib/modules/fs.md](stdlib/modules/fs.md)（manifest から生成）です。

Current selfhost wrapperでrepository rootをpreopenして実行する完全command:

```bash
scripts/run/arukellt-selfhost.sh run tests/fixtures/quickstart/fs_read.ark
```

このwrapperは`wasmtime --dir=<repository-root>`相当を設定します。preopenなしの
runtimeではfilesystem accessは拒否されます。現行`arukellt run`にはuser-facing
`--dir` flagがないため、別directoryを許可するcopy-and-run contractは未提供です。

## 文字列化

`to_string(x)` を基準に使うのが一番安定です。`i32_to_string` などの型別 helper は互換用として残っています。

## Random

<!-- fixture: quickstart/clock_random.ark -->
```ark
use std::random
use std::host::stdio

fn main() {
    let r = random::seeded_random(1)
    stdio::println(to_string(r))
}
```

> `std::host::clock::monotonic_now()` は `i64` を返しますが、現行 selfhost で
> `to_string` 経由の出力に制約があるため、この例では `std::random::seeded_random`（決定論的）のみ使用しています。

## v1 features

このブランチでは v1 系の一部機能も使えますが、まずは上の関数呼び出しスタイルを基準にするのが安全です。
必要なら以下を参照してください。

- [language/syntax.md](language/syntax.md)
- [history/language/syntax-v1-preview.md](history/language/syntax-v1-preview.md)（退役メモ）
- [current-state.md](current-state.md)
