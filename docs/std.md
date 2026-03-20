# Arukellt Standard Surface

The current v0.0.1 standard surface is intentionally narrow. It is centered on builtins and host integrations that are already exercised by the interpreter.

## Target Support Matrix

Use this table when choosing `arktc build --target ...` or `chef build --target ...`.
[`example/meta/matrix.json`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/example/meta/matrix.json) is the example-level contract; this section is the API-level contract.

| surface | interpreter | `wasm-js` | `wasm-wasi` | notes |
| --- | --- | --- | --- | --- |
| `console.println(String)` | yes | yes | yes | `wasm-js` uses a minimal host `console.println(ptr, len)` bridge; `wasm-wasi` lowers to WASI `fd_write` |
| `fs.read_text(path)` | yes | no | yes | file I/O stays capability-scoped |
| `stdin.read_text()` | yes | no | yes | full stdin ingestion on `wasm-wasi` |
| `stdin.read_line()` | yes | no | yes | `wasm-wasi` reads incrementally and trims trailing `\n` / `\r` like the interpreter |
| `string(i64)` | yes | yes | yes | canonical integer-to-string conversion |
| `len(text)` | yes | yes | yes | current surface also accepts `len(list)`; string length is byte length |
| `ends_with_at(text, suffix, end)` | yes | yes | yes | allows suffix checks without allocating substrings |
| `text.split_whitespace()` | yes | yes | yes | whitespace tokenization lowers on both WASM targets |
| `strip_suffix(text, suffix)` | yes | no | yes | current WASI support is enough for ABS-style suffix parsing; `wasm-js` still rejects it |
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
  match fs.read_text("meta/hello.txt"):
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

## Interactive REPL

`arkli` provides a GHCi-style interactive interpreter. Each expression is wrapped in a synthetic function and re-evaluated against the accumulated session source, so bindings defined earlier remain in scope.

```
$ arkli
> 1 + 1
2
> fn double(n: Int) -> Int: n * 2
> double(21)
42
```

## Chef Build

`chef build` mirrors `arktc build` with the same `--target`, `--emit`, and `--output` flags. Use it inside a project managed by a `Chef.toml` to pick up workspace paths automatically.

```
chef build --target wasm-js --emit wat --output out.wat src/main.ar
```

## WASM Boundary

Both WASM targets now cover most of the interpreter surface described in the Target Support Matrix above. Use `--target wasm-js` for a JavaScript host and `--target wasm-wasi` for a WASI runtime.

The output format is controlled separately with `--emit`:

| `--emit` | description |
| --- | --- |
| `wasm` | binary `.wasm` file (default) |
| `wat` | human-readable WebAssembly Text format |
| `wat-min` | minified WAT, collapsed to a single line |

### wasm-js examples

The `wasm-js` target exports compiled functions by their source names instead of requiring a WASI entrypoint.

<!-- snippet: std-wasm-scalar -->
```arukel
fn main() -> Int:
  42
```

<!-- snippet: std-wasm-js-scalar -->
```arukel
fn double(n: Int) -> Int:
  n * 2
```

User-defined fieldless ADTs also lower on this target. Variants become numeric tags, and `match` is supported when each arm matches only a variant name or a final wildcard.

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

### wasm-wasi examples

The `wasm-wasi` target supports `console.println` with string literals, the `string` builtin for integer-to-string conversion, and iterator materialization through `iter.unfold(...).take(n)`.

<!-- snippet: std-wasm-wasi-console -->
```arukel
import console

fn main():
  "hello wasm" |> console.println
```

<!-- snippet: std-wasm-wasi-iter -->
```arukel
fn main() -> Seq<Int>:
  iter.unfold(0, n -> Next(n, n + 1))
```

Any API marked `no` in the Target Support Matrix above will fail hard during `arktc build` with an unsupported-call error.
