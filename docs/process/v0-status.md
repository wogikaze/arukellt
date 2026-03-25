# v0 Implementation Status

> **Last updated**: 2026-03-25
> **Branch**: `feature/arukellt-v1`
> **Test results**: 82 unit tests pass, ~61/124 fixture tests pass

This document is the **single source of truth** for what is actually implemented vs. what is designed/planned in v0. Other docs in this repository describe the *design intent*; this document describes the *current reality*.

## Stage Definitions

| Stage | Meaning |
|-------|---------|
| **designed** | Specified in docs but no compiler code exists |
| **parsed** | Parser accepts the syntax and produces AST |
| **typed** | Type checker validates and infers types |
| **lowered** | MIR lowering produces intermediate representation |
| **emitted** | Wasm code generation produces valid Wasm bytecode |
| **runnable** | End-to-end: compiles and executes correctly via `arukellt run` |

## Backend Reality

| Aspect | Design Intent (docs) | Current Implementation |
|--------|---------------------|----------------------|
| **Primary target** | Wasm GC (struct/array/ref types) | **wasm32 linear memory + WASI Preview 1** |
| **Memory model** | GC-managed reference types | **Bump allocator in linear memory** (structs at heap_ptr, strings length-prefixed) |
| **WASI version** | WASI p1 + p2 (Component Model) | **WASI p1 only** (fd_write for I/O) |
| **Runtime** | wasmtime / V8 / SpiderMonkey | **wasmtime 29 (embedded via Rust crate)** |
| **String repr** | GC array (UTF-8) | **Length-prefixed bytes in linear memory** `[len:4][data:N]` |
| **Struct repr** | GC struct types | **Flat i32 fields in linear memory** (4 bytes per field) |
| **Enum repr** | Tagged union (GC) | **Integer tag only** (unit variants); payload variants not implemented |
| **Vec repr** | GC array with capacity | **Not implemented** |

## Language Features

| Feature | Stage | Notes |
|---------|-------|-------|
| `i32` type | **runnable** | Full arithmetic, comparison, printing |
| `i64` type | **emitted** | Literals emit `i64.const`; no i64-specific arithmetic helpers |
| `f32` type | **emitted** | Literals emit `f32.const`; no f32 arithmetic in emitter |
| `f64` type | **emitted** | Literals emit `f64.const`; no f64_to_string helper |
| `bool` type | **runnable** | `true`/`false`, `bool_to_string`, `print_bool_ln` |
| `char` type | **emitted** | Stored as i32; `char_to_string` prints single byte |
| `String` type | **runnable** | Literal strings, `String_from`, `eq`, `concat` (partial), `println` |
| Tuples | **typed** | Parser + typechecker handle; lowering emits `Unit` (not implemented) |
| Arrays | **typed** | Parser + typechecker handle; lowering emits `Unit` |

## Compound Types

| Feature | Stage | Notes |
|---------|-------|-------|
| `struct` definition | **runnable** | Fields stored in linear memory; all fields treated as i32/ptr |
| `struct` field access | **runnable** | `p.x` loads from memory at field offset |
| `struct` string fields | **runnable** | String field dispatch for `println` works |
| `enum` (unit variants) | **runnable** | Variants as integer tags; match works |
| `enum` (tuple variants) | **parsed** | `Some(42)`, `Err(e)` parsed but payload not lowered |
| `enum` (struct variants) | **parsed** | Parsed but not lowered |
| `Option<T>` | **parsed** | Type registered; `Some`/`None` in prelude but `Some(val)` needs payload variants |
| `Result<T, E>` | **parsed** | Type registered; `Ok`/`Err` in prelude but payload variants not lowered |

## Control Flow

| Feature | Stage | Notes |
|---------|-------|-------|
| `if` / `else` | **runnable** | Both statement and expression forms |
| `while` | **runnable** | With `break` and `continue` |
| `loop` | **runnable** | Infinite loop with `break` / `continue` |
| `loop` as expression | **parsed** | Parser emits `Stmt::Loop` only, not `Expr::Loop` |
| `for` loops | **not implemented** | Deliberately excluded from v0; parser rejects with E0303 |
| `match` (int literals) | **runnable** | Lowered to nested if-else chains |
| `match` (bool literals) | **runnable** | |
| `match` (enum variants) | **runnable** | Unit variants only; payload binding not lowered |
| `match` (wildcard `_`) | **runnable** | |
| `match` (binding `name`) | **runnable** | |
| `match` (tuple patterns) | **parsed** | Not lowered |
| `break` / `continue` | **runnable** | Depth-tracked for nested loops + if blocks |
| `return` (early) | **runnable** | |

## Functions

| Feature | Stage | Notes |
|---------|-------|-------|
| Basic functions | **runnable** | Up to 2 params (emitter type index limit) |
| 3+ param functions | **emitted** | Falls back to `()->()` type index — **will crash at runtime** |
| Recursive functions | **runnable** | Fibonacci etc. work |
| Generic functions | **typed** | No monomorphization; hardcoded type-specific variants only |
| Closures | **parsed** | `\|x\| x + 1` parsed; not typed, not lowered |
| Higher-order functions | **parsed** | Function types in signatures; no `call_indirect` |
| `?` operator | **parsed** | Parsed as `Expr::Try`; not typed, not lowered |

## Operators

| Feature | Stage | Notes |
|---------|-------|-------|
| Arithmetic (`+`, `-`, `*`, `/`, `%`) | **runnable** | i32 only |
| Comparison (`==`, `!=`, `<`, `>`, `<=`, `>=`) | **runnable** | i32 only |
| Logical (`&&`, `\|\|`, `!`) | **runnable** | Short-circuit evaluation |
| Bitwise (`&`, `\|`, `^`, `~`, `<<`, `>>`) | **runnable** | |
| String equality (`eq(a, b)`) | **runnable** | Byte-by-byte comparison |

## Standard Library

| API | Stage | Notes |
|-----|-------|-------|
| `println` / `print` / `eprintln` | **runnable** | WASI fd_write to stdout/stderr |
| `i32_to_string` | **runnable** | Wasm helper function |
| `bool_to_string` | **runnable** | Wasm helper function |
| `char_to_string` | **runnable** | Single byte write |
| `String_from("lit")` | **runnable** | Allocates length-prefixed string |
| `eq(a, b)` | **runnable** | String byte comparison |
| `concat(a, b)` | **designed** | Name resolved; no Wasm implementation |
| `i64_to_string` / `f64_to_string` | **designed** | Registered in typechecker; no Wasm helpers |
| `parse_i32` / `parse_i64` / `parse_f64` | **designed** | Registered; no implementation |
| `Vec_new_i32` / `push` / `pop` / `get` / `set` | **designed** | Names in prelude; no Vec runtime |
| `len` (Vec) | **designed** | Name resolved; no implementation |
| `map_i32_i32` / `filter_i32` / `fold_i32_i32` | **designed** | Names in prelude; requires closures |
| `sort_i32` / `sort_String` | **designed** | Names in prelude; no implementation |
| `slice` / `split` / `join` | **designed** | Names in prelude; no implementation |
| `String_new` / `push_char` / `to_lower` / `to_upper` | **designed** | Names in prelude; no implementation |
| `starts_with` / `ends_with` | **designed** | Names in prelude; no implementation |
| `unwrap` / `unwrap_or` / `is_some` / `is_none` | **designed** | Names in prelude; requires payload variants |
| `sqrt` / `abs` / `min` / `max` | **designed** | Names in prelude; no implementation |
| `clone` | **designed** | Name in prelude; no implementation |
| `panic` | **designed** | Name in prelude; no implementation |
| Capability-based I/O (`fs_read_file`, `fs_write_file`) | **designed** | Not in prelude; not implemented |
| `io/clock` / `io/random` | **designed** | Not implemented |

## Module System

| Feature | Stage | Notes |
|---------|-------|-------|
| `import` syntax | **parsed** | Parser produces `Import` AST nodes |
| Import resolution | **designed** | `resolve.rs` has TODO on line 140 |
| Stdlib auto-import | **partial** | Prelude names injected; no actual module loading |
| User module imports | **not implemented** | |
| Circular import detection | **not implemented** | |

## Toolchain

| Feature | Stage | Notes |
|---------|-------|-------|
| `arukellt compile` | **runnable** | Produces `.wasm` files |
| `arukellt run` | **runnable** | Compiles + executes via embedded wasmtime |
| `arukellt check` | **runnable** | Runs parser + resolver + typechecker |
| Multiple error reporting | **runnable** | DiagnosticSink collects errors; ariadne renders |
| Wasm binary output | **runnable** | WASI p1 compatible modules |
| wasm-gc target | **designed** | Not implemented; linear memory used |
| wasm32-wasi target | **designed** | Documented but not a separate mode |
| WASI p2 / Component Model | **designed** | Not implemented |

## Diagnostics

| Code | Category | Stage |
|------|----------|-------|
| E0001 | Unexpected token | **runnable** |
| E0002 | Missing token | **runnable** |
| E0100 | Unresolved name | **runnable** |
| E0101 | Duplicate definition | **designed** |
| E0200 | Type mismatch | **runnable** |
| E0201-E0206 | Type errors | **partial** — some checked, some designed |
| E0301 | Method syntax rejected | **runnable** |
| E0302 | Nested generics rejected | **runnable** |
| E0303 | `for` loop rejected | **runnable** |
| E0304 | Operator overload rejected | **designed** |
| W0001 | Mutable sharing warning | **designed** |

## Known Limitations

1. **Function arity**: Emitter only has type indices for 0-2 params. Functions with 3+ params get wrong type index and crash.
2. **All values are i32**: The emitter treats all values (including f64, i64) as i32 for arithmetic operations.
3. **No heap deallocation**: Bump allocator never frees memory.
4. **String data region**: Static strings occupy 256-4095; heap starts at 4096. Programs with >3840 bytes of string literals will overflow.
5. **No tail-call optimization**: Deep recursion will overflow Wasm stack.
6. **Silent failures**: Many unsupported features silently emit `i32.const(0)` or `Operand::Unit` instead of producing an error.
