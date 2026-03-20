# Arukellt Language Tour

Arukellt v0.0.1 is an expression-first, indentation-sensitive language aimed at small, recoverable programs. The current public workflow is:

- `arktc check` for syntax/typechecking
- `chef run` for interpreter execution
- `chef test` for executable examples and inline tests
- `arktc build --target wasm-js|wasm-wasi` / `chef build` for WebAssembly output
- `arkli` for interactive REPL exploration

## Hello World

<!-- snippet: language-tour-hello-world -->
```arukel
import console

fn main():
  "Hello, world!" |> console.println
```

## Pure Functions

<!-- snippet: language-tour-pure-max -->
```arukel
fn max(a: Int, b: Int) -> Int:
  if a > b:
    a
  else:
    b
```

## ADTs and Match

<!-- snippet: language-tour-adt-match -->
```arukel
type Choice =
  Left(value: Int)
  Right(value: Int)

fn pick(choice: Choice) -> Int:
  match choice:
    Left(value) -> value
    Right(value) -> value
```

## Structured Diagnostics

`if` is an expression and must include an `else` branch. The compiler reports a structured diagnostic instead of silently accepting a partial form.

<!-- snippet: language-tour-missing-else -->
```arukel
fn main(flag: Bool) -> Int:
  if flag:
    1
```
