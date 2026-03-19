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
Each example has an adjacent `.stdout` fixture that acts as the snapshot contract for the current toolchain.
Each bundled example also passes `arktc check`.
The machine-checkable source of truth for the bundled-example contract lives in [`example/matrix.json`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/example/matrix.json).
After changing a bundled example or extending backend support, refresh the contract by updating that file and rerunning `cargo test -p arktc -p chef --test examples`.

The current bundled-example matrix is:

| example | `chef run` | `chef test` | `arktc check` | `arktc build --target wasm-js` | `arktc build --target wasm-wasi` |
| --- | --- | --- | --- | --- | --- |
| `closure.ar` | pass | pass | pass | fail | fail |
| `factorial.ar` | pass | pass | pass | fail | fail |
| `fibonacci.ar` | pass | pass | pass | fail | fail |
| `file_read.ar` | pass | pass | pass | fail | fail |
| `fizz_buzz.ar` | pass | pass | pass | fail | fail |
| `hello_world.ar` | pass | pass | pass | fail | fail |
| `infinite_iter.ar` | pass | pass | pass | fail | fail |
| `map_filter_sum.ar` | pass | pass | pass | fail | fail |
| `powers.ar` | pass | pass | pass | fail | fail |
| `result_error_handling.ar` | pass | pass | pass | fail | fail |
| `wasm_scalar.ar` | pass | pass | pass | pass | pass |

Only `wasm_scalar.ar` currently fits the bundled WASM subset end to end. The backend also accepts constrained synthetic modules with literal-only `String` returns and fieldless user-defined ADTs plus binding-free `match`, but the rest of the bundled examples still fail on at least one WASM target because they depend on host calls or richer surface features that are not lowered yet.

For release-facing reference material, see the executable docs in [`docs/language-tour.md`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/docs/language-tour.md) and [`docs/std.md`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/docs/std.md). Their snippets are backed by checked-in fixtures and exercised by the test suite.

## Tooling

The public CLI surface is split across `arktc`, `chef`, `arktfmt`, and `arktup`.
Each public binary and subcommand also exposes a tested `--help` path that describes the current prototype contract, including intentionally limited surfaces such as the WASM subset, JSON-only docs output, and local-state-only toolchain management.

### Check

```bash
cargo run -p arktc -- check path/to/file.ar --json
```

This compiles the source through `lang-core` and prints structured diagnostics. The JSON payload includes versioned fields such as `code`, `stage`, `message`, `expected`, `actual`, `cause`, `suggested_fix`, `alternatives`, and `confidence`.

### Run

```bash
cargo run -p chef -- run path/to/file.ar --function main --args 3 9 --step
```

This runs the interpreter path and can optionally print a trace. The interpreter is the default development loop because it is faster to diagnose than the WASM backend.

### Test

```bash
cargo run -p chef -- test path/to/file.ar
cargo run -p chef -- test path/to/file.ar --json
```

Functions whose names start with `test_` are executed and must return `Bool(true)`.
If a file does not define any `test_` functions, `chef test` falls back to snapshot testing against the adjacent `.stdout` fixture.
`--json` emits a versioned result payload listing discovered test names and any failures; compile failures also surface as structured diagnostics JSON.
Without `--json`, compile failures still print actionable human-readable diagnostics before exiting non-zero.

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
If the source does not compile, `arktdoc` exits non-zero and prints a short failure message instead of partial docs.

### Build

```bash
cargo run -p arktc -- build path/to/file.ar --target wasm-js --output out.wasm
cargo run -p arktc -- build path/to/file.ar --target wasm-wasi --output out.wasm
```

The current WASM backend supports only a narrow scalar-plus-literal-string-plus-fieldless-ADT subset.
`wasm-js` emits an embeddable module that exports compiled functions by their Arukel names.
`wasm-wasi` emits a command-style module that exports only `_start`; it requires a zero-argument `main` function and drops any scalar return value at the ABI boundary.
`String` currently lowers only as a raw `i32` pointer into exported read-only `memory` containing NUL-terminated UTF-8 literals. Literal expressions and direct returns through user-defined functions are supported in that ABI slice.
Fieldless user-defined ADTs currently lower as raw numeric tags, and `match` lowers only when the subject is one of those ADTs and each arm is either a bare variant name or a final wildcard.
Unsupported surface does not degrade silently: `arktc build` fails with a hard error as soon as codegen encounters unsupported types or constructs such as string builtins and operations, payload-bearing ADTs, pattern bindings, closures, iterators, or host calls like `console.println`.

### Benchmark

```bash
cargo run -p chef -- benchmark benchmarks/pure_logic.json
```

This reports parse, typecheck, execution, and pass counts for a JSON benchmark manifest. The sample manifest at [`benchmarks/pure_logic.json`](/home/wogikaze/arukellt/.worktrees/arukellt-v0/benchmarks/pure_logic.json) is the current reference set.

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
- The WASM backend supports only literal-only `String` lowering via read-only memory; string builtins, string operations, and general string ABI tooling are still unsupported
- The WASM backend supports only fieldless user-defined ADTs and binding-free `match`; payload-bearing constructors and pattern bindings are still unsupported, along with closures, iterators, and host call codegen
- Host integrations are currently limited to the example-oriented `console` and `fs` interpreter shims
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

This project is currently validated primarily through:

- core pipeline tests
- IR lowering tests
- interpreter evaluation tests
- CLI integration tests
- playground API tests
- benchmark loop smoke tests
