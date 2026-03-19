# Arukellt Standard Surface

The current v0.0.1 standard surface is intentionally narrow. It is centered on builtins and host integrations that are already exercised by the interpreter.

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

The `wasm-wasi` target also supports `console.println` with string literals and the `string` builtin for integer-to-string conversion.

<!-- snippet: std-wasm-wasi-console -->
```arukel
import console

fn main():
  "hello wasm" |> console.println
```

Anything outside that subset should fail hard during `arktc build`. Payload-bearing constructors and iterator helpers remain unsupported, and closures currently lower only on `wasm-wasi`.

<!-- snippet: std-wasm-unsupported -->
```arukel
fn main() -> Seq<Int>:
  iter.unfold(0, n -> Next(n, n + 1))
```
