# v0 Implementation Status

> **Last updated**: 2026-03-25
> **Branch**: `feature/arukellt-v1`
> **Test results**: 81 unit tests pass, 127/135 fixture tests pass (8 skipped — modules not yet wired)

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
| `i64` type | **runnable** | Literals, arithmetic, `i64_to_string` |
| `f32` type | **emitted** | Literals emit `f32.const`; no f32 arithmetic helpers |
| `f64` type | **runnable** | Literals, arithmetic, `f64_to_string` |
| `bool` type | **runnable** | `true`/`false`, `bool_to_string`, `print_bool_ln` |
| `char` type | **runnable** | Stored as i32; `char_to_string` prints single byte |
| `String` type | **runnable** | Literals, `String_from`, `eq`, `concat`, `len`, `split`, `join`, `slice`, `starts_with`, `ends_with`, `println` |
| Tuples | **runnable** | Tuple literals `(a, b)`, destructuring `let (x, y) = ...`, return types |
| Arrays | **parsed** | Parser + typechecker handle; Wasm emission crashes at runtime |

## Compound Types

| Feature | Stage | Notes |
|---------|-------|-------|
| `struct` definition | **runnable** | Fields stored in linear memory; all fields treated as i32/ptr |
| `struct` field access | **runnable** | `p.x` loads from memory at field offset |
| `struct` string fields | **runnable** | String field dispatch for `println` works |
| `enum` (unit variants) | **runnable** | Variants as integer tags; match works |
| `enum` (tuple variants) | **runnable** | `Some(42)`, `Ok(val)`, `Err(e)` — payload binding works |
| `enum` (struct variants) | **parsed** | Parsed but `Variant { field: val }` syntax not yet emitted |
| `Option<T>` | **runnable** | `Some(val)` / `None`; `is_some`, `is_none`, `unwrap`, `unwrap_or`, `option_map` all work |
| `Result<T, E>` | **runnable** | `Ok(val)` / `Err(e)`; match and `?` operator work |

## Control Flow

| Feature | Stage | Notes |
|---------|-------|-------|
| `if` / `else` | **runnable** | Both statement and expression forms |
| `while` | **runnable** | With `break` and `continue` |
| `loop` | **runnable** | Infinite loop with `break` / `continue` |
| `loop` as expression | **runnable** | `let x = loop { break value }` works |
| `for` loops | **runnable** | `for i in a..b` (range) and `for x in values(v)` (Vec iteration) |
| `match` (int literals) | **runnable** | Lowered to nested if-else chains |
| `match` (bool literals) | **runnable** | |
| `match` (enum variants) | **runnable** | Unit variants only; payload binding not lowered |
| `match` (wildcard `_`) | **runnable** | |
| `match` (binding `name`) | **runnable** | |
| `match` (tuple patterns) | **runnable** | Tuple destructuring in match arms works |
| `break` / `continue` | **runnable** | Depth-tracked for nested loops + if blocks |
| `return` (early) | **runnable** | |

## Functions

| Feature | Stage | Notes |
|---------|-------|-------|
| Basic functions | **runnable** | Arbitrary param count supported |
| 3+ param functions | **runnable** | Correct type indices generated for all arities |
| Recursive functions | **runnable** | Fibonacci etc. work |
| Generic functions | **runnable** | Monomorphized; `fn id<T>(x: T) -> T`, multi-param generics (`<A, B>`) work |
| Closures | **runnable** | `\|x\| expr` lambda syntax works; captures not supported |
| Higher-order functions | **runnable** | Function references as arguments (e.g. `map_i32_i32(v, double)`) |
| `?` operator | **runnable** | Early-return on `Err`/`None`; works in functions returning `Result` or `Option` |

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
| `i64_to_string` | **runnable** | Wasm helper function |
| `f64_to_string` | **runnable** | Wasm helper function |
| `bool_to_string` | **runnable** | Wasm helper function |
| `char_to_string` | **runnable** | Single byte write |
| `String_from("lit")` | **runnable** | Allocates length-prefixed string |
| `String_new()` | **runnable** | Creates empty string |
| `is_empty(s)` | **runnable** | Returns `true` if length is 0 |
| `len(s)` | **runnable** | String byte length |
| `eq(a, b)` | **runnable** | String byte comparison |
| `concat(a, b)` | **runnable** | Concatenates two strings |
| `slice(s, start, end)` | **runnable** | Returns substring |
| `split(s, sep)` | **runnable** | Returns `Vec<String>` |
| `join(v, sep)` | **runnable** | Joins `Vec<String>` with separator |
| `starts_with(s, prefix)` | **runnable** | Prefix check |
| `ends_with(s, suffix)` | **runnable** | Suffix check |
| `push_char(s, c)` | **designed** | Not yet in prelude |
| `to_lower(s)` / `to_upper(s)` | **designed** | Not yet in prelude |
| `parse_i32(s)` | **runnable** | Returns `Result<i32, String>`; use `?` or `match` |
| `parse_i64` / `parse_f64` | **designed** | Registered; no implementation |
| `Vec_new_i32()` | **runnable** | Creates empty `Vec<i32>` |
| `push(v, x)` | **runnable** | Appends element |
| `pop(v)` | **runnable** | Removes and returns last element as `Option<T>` |
| `get_unchecked(v, i)` | **runnable** | Index without bounds check |
| `set(v, i, x)` | **runnable** | Sets element at index |
| `len(v)` | **runnable** | Vec element count |
| `values(v)` | **runnable** | Iterator over Vec for use in `for x in values(v)` |
| `map_i32_i32(v, f)` | **runnable** | Maps function over Vec |
| `filter_i32(v, f)` | **runnable** | Filters Vec by predicate |
| `fold_i32_i32(v, init, f)` | **runnable** | Folds Vec with accumulator |
| `sort_i32(v)` | **runnable** | Sorts Vec in-place |
| `sort_String(v)` | **designed** | Not yet implemented |
| `is_some(opt)` / `is_none(opt)` | **runnable** | Option predicates |
| `unwrap(opt)` | **runnable** | Extracts `Some` value (panics on `None` at runtime) |
| `unwrap_or(opt, default)` | **runnable** | Extracts value or returns default |
| `option_map(opt, f)` | **runnable** | Maps function over `Option` |
| `sqrt` / `abs` / `min` / `max` | **designed** | Not yet in prelude |
| `clone` | **designed** | Not yet in prelude |
| `panic` | **designed** | Not yet in prelude |
| String interpolation `f"..."` | **runnable** | `f"text {expr}"` — expressions interpolated at runtime |
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
| E0303 | `for` loop rejected | **removed** — `for` loops are now implemented |
| E0304 | Operator overload rejected | **designed** |
| W0001 | Mutable sharing warning | **designed** |

## Known Limitations

1. **Arrays**: Array literals and indexing parse and type-check but produce a Wasm compile error at runtime — not yet emitted.
2. **Closures — no captures**: Lambda syntax `|x| expr` works for single-expression bodies, but closures cannot capture variables from the enclosing scope.
3. **Enum struct variants**: `Variant { field: val }` construction and `Variant { field }` destructuring in `match` are parsed but not emitted.
4. **No heap deallocation**: Bump allocator never frees memory.
5. **String data region**: Static strings occupy 256–4095; heap starts at 4096. Programs with >3840 bytes of string literals will overflow.
6. **No tail-call optimization**: Deep recursion will overflow the Wasm stack.
7. **Silent failures**: Some unsupported features silently emit `i32.const(0)` or `Operand::Unit` instead of producing an error.
8. **`f32` not fully integrated**: `f32` literals emit `f32.const`, but no `f32_to_string` or arithmetic helpers exist.
