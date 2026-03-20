# Arukellt

Arukellt is an experimental LLM-first language toolchain implemented in Rust.
The current `v0` prototype is optimized for small pure-logic programs that are easy for an LLM to generate, repair, and validate. The pipeline is expression-first, immutable by default, and designed around structured diagnostics instead of opaque compiler failures.

## Current Status

The repository already contains a working vertical slice:

- `lang-core`: lexer, tolerant indent-aware parser, AST, typed AST, and structured diagnostics
- `lang-ir`: High IR and Low IR lowering
- `lang-interp`: interpreter for the typed/high-level subset
- `lang-backend-wasm`: WASM backend for the current scalar, literal-string, and fieldless-ADT subset
- `arktc`: compiler-facing `check` and `build` commands
- `chef`: interpreter-facing `run`, `test`, and `benchmark` commands
- `arkli`: interactive REPL for ad hoc evaluation and file loading
- `arktfmt`: source formatter
- `arktup`: local toolchain-state manager for prototype installs and default selection
- `lang-playground-core`: JSON/wasm-bindgen API for browser playground integration

This is still a prototype. The language and toolchain are intentionally incomplete.

## Target Surface

The target `v0` surface is centered on clarity, recoverability, and low-ambiguity generation for LLMs:

- Indentation-based blocks
- Top-level order: `import` -> `type` -> `fn`
- Host effects imported by name, for example `import console` and `import fs`
- `i64` as the default integer type in user-facing examples
- Function types written as `Fn<arg, result>` to avoid overloading `->`
- Expression-first `if` with mandatory `else`
- ADTs with constructor calls
- `match` with wildcard warnings and exhaustiveness checks
- Range literals such as `1..=100`
- Lambda shorthand such as `n -> expr`, with callback parameter types inferred from context
- Method chains stay pure; side effects happen at the boundary through trailing pipe, for example `value |> console.println`
- Structured diagnostics with stable JSON schema version `v0.1`
- No `null`, exceptions, macros, implicit coercions, or shared mutable state

Example:

```text
import console

type Error =
  DivisionByZero

fn divide(a: i64, b: i64) -> Result<i64, Error>:
  if b == 0:
    Err(DivisionByZero)
  else:
    Ok(a / b)

fn render_error(error: Error) -> String:
  match error:
    DivisionByZero -> "error"

fn main():
  match divide(10, 0):
    Ok(value) -> value |> string |> console.println
    Err(error) -> error |> render_error |> console.println
```

## Examples

The repository includes language-surface examples in [`example/`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/example):

- hello world
- fizz buzz over `1..=100`, keeping the chain pure until the final `|> console.println`
- factorial and fibonacci
- `map` / `filter` / `sum`
- file reads and `Result`-based error handling
- closures
- infinite iterators
- a pure scalar WASM-friendly slice

All bundled examples are executable through `chef run` and verifiable through `chef test`.
Each example has a matching fixture under [`example/meta/`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/example/meta) that acts as the snapshot contract for the current toolchain.
Each bundled example also passes `arktc check`.
The machine-checkable source of truth for the bundled-example contract lives in [`example/meta/matrix.json`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/example/meta/matrix.json).
After changing a bundled example or extending backend support, refresh the contract by updating that file and rerunning `cargo test -p arktc -p chef --test examples`.

The current bundled-example matrix is:

| example | `chef run` | `chef test` | `arktc check` | `arktc build --target wasm-js` | `arktc build --target wasm-wasi` |
| --- | --- | --- | --- | --- | --- |
| `closure.ar` | pass | pass | pass | pass | pass |
| `factorial.ar` | pass | pass | pass | pass | pass |
| `fibonacci.ar` | pass | pass | pass | pass | pass |
| `file_read.ar` | pass | pass | pass | fail | pass |
| `fizz_buzz.ar` | pass | pass | pass | pass | pass |
| `hello_world.ar` | pass | pass | pass | pass | pass |
| `infinite_iter.ar` | pass | pass | pass | pass | pass |
| `map_filter_sum.ar` | pass | pass | pass | pass | pass |
| `powers.ar` | pass | pass | pass | pass | pass |
| `result_error_handling.ar` | pass | pass | pass | pass | pass |
| `wasm_scalar.ar` | pass | pass | pass | pass | pass |

`wasm-wasi` now builds every bundled example in the repository. `wasm-js` also builds every bundled example except `file_read.ar`, so the remaining cross-target gap in the example set is host file I/O rather than lists, iterators, or string output.

For release-facing reference material, see the executable docs in [`docs/language-tour.md`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/docs/language-tour.md) and [`docs/std.md`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/docs/std.md). Their snippets are backed by checked-in fixtures and exercised by the test suite.

## Tooling

The public CLI surface is split across `arktc`, `chef`, `arkli`, `arktfmt`, and `arktup`.
Each public binary and subcommand also exposes a tested `--help` path that describes the current prototype contract, including intentionally limited surfaces such as the WASM subset, JSON-only docs output, and local-state-only toolchain management.

### Check

```bash
cargo run -p arktc -- check path/to/file.ar --json
```

This compiles the source through `lang-core` and prints structured diagnostics. The JSON payload includes versioned fields such as `code`, `stage`, `message`, `expected`, `actual`, `cause`, `suggested_fix`, `alternatives`, and `confidence`.

### Run

```bash
cargo run -p chef -- run path/to/file.ar --function main --args 3 9 --step
printf '1\n2 3\ntest\n' | cargo run -p chef -- run path/to/practicea.ar
```

This runs the interpreter path and can optionally print a trace. The interpreter is the default development loop because it is faster to diagnose than the WASM backend.
If the program calls `stdin.read_text()`, `chef run` reads from the invoking process stdin, so competitive-programming style input can be piped in directly.
If compilation fails before execution starts, `chef run` exits non-zero and writes structured diagnostics JSON to stderr.

### Test

```bash
cargo run -p chef -- test path/to/file.ar
cargo run -p chef -- test path/to/file.ar --json
```

Functions whose names start with `test_` are executed and must return `Bool(true)`.
If a file does not define any `test_` functions, `chef test` falls back to snapshot testing against the adjacent `.stdout` fixture.

```bash
cargo run -p arkli
```

`arkli` provides a minimal GHCi-style REPL. It evaluates one-line expressions, keeps interactive `let` bindings for the current session, supports `:load path/to/file.ar`, `:reload`, `:type <expr>`, and exits via `:quit` or `:q`.
`--json` emits a versioned result payload listing discovered test names and any failures; compile failures surface as structured diagnostics JSON on stderr.
Without `--json`, compile failures still print actionable human-readable diagnostics on stderr before exiting non-zero.

### Format

```bash
cargo run -p arktfmt -- path/to/file.ar
cargo run -p arktfmt -- path/to/file.ar --write
```

This formats the parsed module and either prints the result to stdout or writes it back to the source file.
If lexer or parser errors are present, `arktfmt` fails explicitly and leaves the input file unchanged instead of emitting placeholder `<error>` nodes.

### Docs

```bash
cargo run -p arktdoc -- path/to/file.ar --format json
```

This compiles a source file and prints versioned JSON documentation for the typed function surface.
The current payload includes the input file path plus each function's name, visibility, parameter list, and return type.
Only `--format json` is currently supported. Other format values fail explicitly instead of silently falling back to JSON.
If the source does not compile, `arktdoc` exits non-zero and prints a short compilation-failure message instead of partial docs, even when an unsupported `--format` value was also passed.

### Build

```bash
cargo run -p arktc -- build path/to/file.ar --target wasm-js --output out.wasm
cargo run -p arktc -- build path/to/file.ar --target wasm-wasi --output out.wasm
cargo run -p arktc -- build path/to/file.ar --target wasm-js-gc --emit wat
cargo run -p arktc -- build path/to/file.ar --target wasm-js --emit wat
cargo run -p arktc -- build path/to/file.ar --target wasm-wasi --emit wat-min
cargo run -p chef -- build path/to/file.ar --target wasm-wasi --output out.wasm
```

The current WASM backend supports a narrow scalar-plus-list-plus-string subset on `wasm-wasi`, and a smaller but now collection-capable `List<i64>` subset on `wasm-js`.
`wasm-js-gc` is now a documented explicit experimental target contract for a future GC-capable JavaScript-host backend, but current builds reject it until that backend exists.
`chef build` now exposes the same target and emit matrix as `arktc build`, which is useful when you want run/test/build workflows under one CLI.
`--target` selects the ABI (`wasm-js`, `wasm-wasi`, or the reserved experimental `wasm-js-gc` contract) while `--emit` selects the output format (`wasm`, `wat`, or `wat-min`).
For API-by-API target coverage, see the target support matrix in [`docs/std.md`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/docs/std.md). The bundled example matrix above is example-level; the std doc is the source of truth for questions such as whether `parse.i64`, `split_whitespace`, or `stdin.read_line` lower on a given WASM target.
`--output` is currently optional; if you omit it, `arktc build` prints WAT for `--emit wat` / `--emit wat-min`, and otherwise discards the generated WASM bytes after successful codegen.
`--target wat` is still accepted as a deprecated alias for `--target wasm-js --emit wat`.
`wasm-js` emits an embeddable module that exports compiled functions by their Arukel names.
`wasm-wasi` emits a command-style module that exports only `_start`; it requires a zero-argument `main` function and drops any scalar return value at the ABI boundary.
`wasm-js-gc` is intentionally separate from `wasm-js`: its first-slice contract is planned as a scalar-only public ABI with internal GC refs allowed inside the module, and current builds reject the target with a contract error until the GC backend lands.
`String` currently lowers only as a raw `i32` pointer into exported read-only `memory` containing NUL-terminated UTF-8 literals. Literal expressions and direct returns through user-defined functions are supported in that ABI slice.
Fieldless user-defined ADTs currently lower as raw numeric tags, and `match` lowers only when the subject is one of those ADTs and each arm is either a bare variant name or a final wildcard.
Unsupported surface does not degrade silently: `arktc build` fails with a hard error as soon as codegen encounters unsupported types or constructs outside the documented subset, such as unsupported string helpers on the selected target, payload-bearing ADTs, pattern bindings, or unsupported host calls, and the error points back to the API-level support matrix in [`docs/std.md`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/docs/std.md).

### Benchmark

```bash
cargo run -p chef -- benchmark benchmarks/pure_logic.json
```

This reports parse, typecheck, execution, and pass counts for a JSON benchmark manifest.
`parse_success` counts cases that get past lexing and parsing without lexer/parser errors, even if typechecking later fails; lexer/parser warnings still count as parse success. `typecheck_success` counts only fully compiled cases.
The sample manifest at [`benchmarks/pure_logic.json`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/benchmarks/pure_logic.json) is the current reference set.

### Toolchain

```bash
cargo run -p arktup -- show
ARKTUP_HOME=/tmp/arktup cargo run -p arktup -- install v0.1.0
ARKTUP_HOME=/tmp/arktup cargo run -p arktup -- default v0.1.0
```

`arktup` currently manages local toolchain metadata only. It records installed versions and the selected default version in `ARKTUP_HOME/state.json`, or in `.arktup/state.json` under the current working directory when `ARKTUP_HOME` is unset.

## Browser Playground API

`crates/lang-playground-core` exposes two JSON-oriented functions for browser integration:

- `analyze_source_json(source)` returns versioned diagnostics JSON
- `run_source_json(source, function, args_json, step)` returns result and optional trace JSON

These are also exported via `wasm-bindgen` as `analyze_source` and `run_source`.

## Repository Layout

```text
.
├── benchmarks/
├── crates/
│   ├── arktc/
│   ├── arkli/
│   ├── arktdoc/
│   ├── arktfmt/
│   ├── arktup/
│   ├── chef/
│   ├── lang-core/
│   ├── lang-ir/
│   ├── lang-interp/
│   ├── lang-backend-wasm/
│   └── lang-playground-core/
└── Cargo.toml
```

## Limitations

The executable prototype is still intentionally narrower than the full language plan:

- `lang-core` still models integers as `Int` internally, while the surface examples use the explicit `i64` spelling
- The bundled examples are executable through the interpreter path, but most of them are intentionally outside the current WASM subset
- The supported standard library remains small and purpose-built around the bundled examples
- The WASM backend hard-fails on unsupported surface instead of emitting placeholder modules
- The WASM backend supports literal strings plus heap-backed strings from `string()`/`join()` on both WASM targets; broader string operations and general string ABI tooling are still unsupported
- The WASM backend supports payload-bearing `Result` values, heap-backed user-defined ADTs, pattern bindings, unary closures, `List<i64>` collection helpers, minimal `Seq<i64>` materialization via `iter.unfold(...).take(n)`, `console.println`, and `fs.read_text` on `wasm-wasi`; broader iterator/host-call codegen is still unsupported
- Host integrations are currently limited to the example-oriented `console.println` and `fs.read_text` shims
- `clock`, `random`, `process`, package management, builders, and a richer standard library are not implemented yet
- `arktfmt` currently preserves source rather than reprinting a canonical AST-based format
- `arktdoc` currently emits JSON only; non-JSON `--format` values are rejected until another output contract is implemented

## Workboard

[`WORKBOARD.md`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/WORKBOARD.md) is the shared AI-managed queue for repository work.
AI agents update it as work starts, blocks, splits, and completes; humans can read it for the current `Next`, `Ready`, `Blocked`, and `Done` state.

## Development

Run the workspace test suite from the repository root:

```bash
cargo test
```

For a browser-level smoke pass of the static docs shell, run:

```bash
cargo test -p arktc --test docs_site docs_site_routes_render_in_headless_browser -- --exact --ignored --nocapture
```

That command starts a local `python3 -m http.server` rooted at `docs/` and uses headless `google-chrome` to verify the `#/language-tour`, `#/std`, and no-hash fallback routes without manual inspection.

This project is currently validated primarily through:

- core pipeline tests
- IR lowering tests
- interpreter evaluation tests
- CLI integration tests
- playground API tests
- benchmark loop smoke tests
