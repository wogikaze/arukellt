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

Anything outside that scalar subset should fail hard during `arktc build`.

<!-- snippet: std-wasm-unsupported -->
```arukel
import console

fn main():
  "hi" |> console.println
```
