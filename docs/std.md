# Arukellt Standard Surface

The current v0.0.1 standard surface is intentionally narrow. It is centered on builtins and host integrations that are already exercised by the interpreter.

## Target Support Matrix

Use this table when choosing `arktc build --target ...` or `chef build --target ...`.
[`example/matrix.json`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/example/matrix.json) is the example-level contract; this section is the API-level contract.

| surface | interpreter | `wasm-js` | `wasm-wasi` | notes |
| --- | --- | --- | --- | --- |
| `console.println(String)` | yes | yes | yes | `wasm-js` uses a minimal host `console.println(ptr, len)` bridge; `wasm-wasi` lowers to WASI `fd_write` |
| `fs.read_text(path)` | yes | no | yes | file I/O stays capability-scoped |
| `stdin.read_text()` | yes | no | yes | full stdin ingestion on `wasm-wasi` |
| `stdin.read_line()` | yes | no | no | interpreter-only for now |
| `string(i64)` | yes | yes | yes | canonical integer-to-string conversion |
| `text.split_whitespace()` | yes | yes | yes | whitespace tokenization lowers on both WASM targets |
| `parse.i64(text)` | yes | yes | yes | pure parse helper available across interpreter and both WASM targets |
| `parse.bool(text)` | yes | yes | yes | same boundary as `parse.i64` |
| `[1, 2, 3]` and `1..=3` as `List<i64>` | yes | yes | yes | current heap-backed WASM list runtime is `List<i64>`-only |
| `list.map(...)`, `list.filter(...)`, `list.sum()` on `List<i64>` | yes | yes | yes | current lowering stays scoped to `List<i64>` |
| `list.join(", ")` | yes | yes | yes | current runtime coverage is string-joining output paths |
| `iter.unfold(...).take(n)` | yes | yes | yes | current lowering materializes `Seq<i64>` through `take` |

If a build fails with an unsupported-call error such as `calls to \`parse.i64\` are not yet supported in wasm backend`, that is expected whenever this table says `no` for the selected target.

## Collection Pipelines and Closures

<!-- snippet: std-closure-map -->
```arukel
import console

fn main():
  [1, 2, 3]
    .map(n -> n * 2)
    .map(string)
    .join(", ")
    |> console.println
```

## File Reads

`fs.read_text` is currently an interpreter-facing capability that returns `Ok(...)` / `Err(...)`.

<!-- snippet: std-file-read -->
```arukel
import console
import fs

fn main():
  match fs.read_text("hello.txt"):
    Ok(text) -> text |> console.println
    Err(_) -> "read failed" |> console.println
```

## Inline Tests

Functions whose names start with `test_` can be executed with `chef test`.

<!-- snippet: std-inline-test -->
```arukel
fn double(n: Int) -> Int:
  n * 2

fn test_double() -> Bool:
  double(21) == 42
```

## WASM Boundary

The current WASM subset is much smaller than the interpreter surface.

<!-- snippet: std-wasm-scalar -->
```arukel
fn main() -> Int:
  42
```

The `wasm-js` target exports compiled functions by their source names instead of requiring a WASI entrypoint.

<!-- snippet: std-wasm-js-scalar -->
```arukel
fn double(n: Int) -> Int:
  n * 2
```

User-defined fieldless ADTs also lower on the current backend. Variants become numeric tags, and `match` is supported when each arm matches only a variant name or a final wildcard.

<!-- snippet: std-wasm-js-fieldless-match -->
```arukel
type Choice =
  Left
  Right

fn choose(flag: Bool) -> Choice:
  if flag:
    Left
  else:
    Right

fn main() -> Int:
  match choose(false):
    Left -> 1
    Right -> 2
```

The `wasm-wasi` target also supports `console.println` with string literals, the `string` builtin for integer-to-string conversion, and minimal iterator materialization through `iter.unfold(...).take(n)`.

<!-- snippet: std-wasm-wasi-console -->
```arukel
import console

fn main():
  "hello wasm" |> console.println
```

Anything outside that subset should fail hard during `arktc build`. Broader host calls and any API marked `no` in the target support matrix above remain unsupported, and closures currently lower only on `wasm-wasi`.

<!-- snippet: std-wasm-wasi-iter -->
```arukel
fn main() -> Seq<Int>:
  iter.unfold(0, n -> Next(n, n + 1))
```
