# Arukellt Language Guide

> **Explanatory**: This document explains concepts and usage patterns.
> It is not the authoritative specification. For normative behavior, see [spec.md](spec.md)
> and [../current-state.md](../current-state.md).

> **Canonical API note**: The canonical surface is methods and associated
> functions (`Vec::new<i32>()`, `v.push(x)`, `x.to_string()`); see ADR-044 / ADR-046.
> Some free-function forms (`push(v, x)`, `to_string(x)`, monomorphic
> constructors like `Vec_new_i32()`) remain available as a compatibility
> surface while they are retired under #917.
> See [../stdlib/migration-guidance.md](../stdlib/migration-guidance.md) for migration status.

This guide covers the **stable, implemented** features of the Arukellt programming language,
with explicit callouts where a **provisional** surface is needed for practical use.
It is written for readers who want a practical introduction before diving into the full
[spec.md](spec.md). Features that are experimental or unimplemented are
not covered here — consult the [Feature Maturity Matrix](maturity-matrix.md) for the
complete stability picture.

> **Code example verification**: Code examples in this guide use two verification paths:
>
> 1. **Fixture-backed** (`<!-- fixture: path.ark -->`): The block is verified by a
>    registered fixture in `tests/fixtures/manifest.txt`. CI compiles and runs the
>    fixture, not the inline block. These examples are exact copies of the fixture source.
> 2. **Skip-doc-check** (HTML comment marker `skip-doc-check` with structured fields):
>    The block is excluded from doc-example compilation because it demonstrates a concept that
>    cannot be compiled standalone (e.g. uses `print` prelude alias not in the bootstrap,
>    shows a type signature without a `main` function, or is pseudocode for a future feature).
>    Each skip has a structured `reason`, `owner` (issue #), `kind`, and `expires` date.
>
> The majority of examples in this guide are currently skip-doc-check (kind=`non-runnable`)
> because they use the `println`/`print` prelude alias which the bootstrap compiler does
> not resolve in standalone mode. The underlying functions (`stdio::println`/`stdio::print`)
> are stable and fixture-backed. See [current-state.md](../current-state.md) for the
> implementation status of each feature.

---

## Table of Contents

1. [Hello, World](#1-hello-world)
2. [Variables and Bindings](#2-variables-and-bindings)
3. [Primitive Types](#3-primitive-types)
4. [Functions](#4-functions)
5. [Control Flow](#5-control-flow)
6. [Structs](#6-structs)
7. [Enums](#7-enums)
8. [Pattern Matching](#8-pattern-matching)
9. [Error Handling](#9-error-handling)
10. [Collections](#10-collections)
11. [Closures](#11-closures)
12. [Generics](#12-generics)
13. [Imports and Modules](#13-imports-and-modules)
14. [Standard Library Quick Reference](#14-standard-library-quick-reference)

---

## 1. Hello, World

Every Arukellt program starts from a `main` function.
Canonical runnable fixture: `tests/fixtures/hello/hello.ark`.

<!-- fixture: hello/hello.ark -->
```text
fn main() {
    println(String_from("Hello, world!"))
}
```

With an exit code:

```ark
use std::host::stdio

fn main() -> i32 {
    stdio::println("Hello, world!")
    0
}
```

Source files are UTF-8. Comments use `//` for line comments and `/* … */` for block comments (nestable). Doc comments use `///` on the item they annotate.

---

## 2. Variables and Bindings

Declare bindings with `let`. By default they are immutable:

<!-- fixture: guide/variables.ark -->
<!-- skip-doc-check reason="binding example awaits a dedicated fixture" owner="#461" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let x = 42
let name = String_from("Alice")
```

Use `mut` to allow reassignment. Any mutation through a binding —
assignment, field/index writes, or mutating method calls — requires the
binding to be declared `let mut` (ADR-059):

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let mut counter = 0
counter = counter + 1
```

Type annotations are optional when the type can be inferred:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let n: i32 = 10
let s: String = String_from("hello")
```

Destructuring in `let`:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let (a, b) = (1, 2)
```

Semicolons are optional statement terminators. The last expression in a block (without a trailing `;`) is the block's return value.

---

## 3. Primitive Types

> 📘 **Canonical reference**: [spec.md §2 Type System](spec.md#2-type-system) · [type-system.md](type-system.md)

| Type | Description | Example |
|------|-------------|---------|
| `i32` | 32-bit signed integer (default) | `42` |
| `i64` | 64-bit signed integer | `42i64` |
| `f64` | 64-bit float (default) | `3.14` |
| `f32` | 32-bit float | `3.14f32` |
| `bool` | Boolean | `true`, `false` |
| `char` | Unicode scalar value | `'a'` |
| `()` | Unit type | `()` |
| `String` | UTF-8 string (reference) | `String_from("hi")` |

**Integer literals** default to `i32`. Add a suffix (`i64`, `u32`, etc.) to change the type.  
**Float literals** default to `f64`.  
**String literals** are `"…"` (double-quoted) with standard escapes (`\\`, `\"`, `\n`, `\r`, `\t`).  
**Interpolated strings**: `f"Hello, {name}!"` — desugared by the compiler to `concat` calls.

### Numeric coercion

When operands of an arithmetic expression differ in type, the narrower type is implicitly widened:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let x: i32 = 1
let y: i64 = 2i64
let z = x + y    // z: i64
```

No implicit narrowing is ever performed — use an explicit conversion function.

---

## 4. Functions

<!-- fixture: guide/fn_add.ark -->
<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

- Return type defaults to `()` when omitted.
- The last expression in the body (without `;`) is the implicit return.
- Use `return expr` for early returns.
- `pub` makes a function visible to importing modules.

Arguments are borrowed for the duration of the call — a call does not move a
non-`Copy` argument out of the caller's binding. Mutating the caller's value
through a parameter requires a `mut` parameter (ADR-088), which is not
implemented yet:

```text
fn bump(mut v: Vec<i32>) {
    v.push(1)        // visible to the caller
}

let mut xs: Vec<i32> = Vec::new<i32>()
bump(xs)           // OK: `xs` is a mutable place
```

See [spec.md §3.5.1](spec.md#351-argument-passing-and-parameter-modes) for the
call-boundary contract and tracking issue #951 for implementation status.

Generic functions (see also §12):

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
fn identity<T>(x: T) -> T {
    x
}
```

---

## 5. Control Flow

> 📘 **Canonical reference**: [spec.md §3.10–3.18](spec.md#310-if-expression) · [syntax.md](syntax.md)

### if / else

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio

if x > 0 {
    stdio::println(String_from("positive"))
} else if x == 0 {
    stdio::println(String_from("zero"))
} else {
    stdio::println(String_from("negative"))
}
```

`if` is an expression — both branches must produce the same type when the value is used:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let label = if x > 0 { String_from("pos") } else { String_from("neg") }
```

### while

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
while n > 0 {
    n = n - 1
}
```

### loop

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
loop {
    if done { break }
}
```

`loop` can return a value via `break expr`:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let result = loop {
    if condition { break 42 }
}
```

### for

Range iteration (half-open `[start, end)`):

<!-- fixture: for_loops/for_range.ark -->
<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio

for i in 0..10 {
    stdio::println((i).to_string())
}
```

Iterate over a `Vec`:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio
use std::core::iter

for item in v {
    stdio::println((item).to_string())
}
```

### break and continue

`break` exits the nearest enclosing loop. `continue` skips to the next iteration.

---

## 6. Structs

Define a named product type:

<!-- fixture: structs/basic_struct.ark -->
<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
struct Point {
    x: i32,
    y: i32,
}
```

Create and access:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio

let p = Point { x: 3, y: 4 }
stdio::println((p.x).to_string())
```

Struct update syntax (copy all fields from `base`, override selected ones):

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let p2 = Point { x: 10, ..p }
```

Generic structs:

```ark
pub struct Pair<A, B> {
    first: A,
    second: B,
}
```

---

## 7. Enums

Enums define a tagged union. Variants can carry data:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
enum Direction {
    North,
    South,
    East,
    West,
}

enum Shape {
    Circle(i32),            // radius
    Rect(i32, i32),         // width, height
    Named { label: String } // struct variant
}
```

Construct a variant:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let d = Direction::North
let s = Shape::Circle(5)
```

The built-in `Option<T>` and `Result<T, E>` follow the same rules:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let maybe: Option<i32> = Some(42)
let ok: Result<i32, String> = Ok(1)
let err: Result<i32, String> = Err(String_from("bad"))
```

`Some`, `None`, `Ok`, `Err`, `Option`, `Result` are in the prelude — no import needed.

---

## 8. Pattern Matching

> 📘 **Canonical reference**: [spec.md §5 Pattern Matching](spec.md#5-pattern-matching)

`match` exhaustively tests a value against a sequence of patterns:

<!-- fixture: control/match_enum.ark -->
<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio

match direction {
    Direction::North => stdio::println(String_from("north")),
    Direction::South => stdio::println(String_from("south")),
    _ => stdio::println(String_from("east or west")),
}
```

Match arms can include optional guards:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio

match x {
    n if n > 0 => stdio::println(String_from("positive")),
    0          => stdio::println(String_from("zero")),
    _          => stdio::println(String_from("negative")),
}
```

### Patterns you can use

| Pattern | Example | Matches |
|---------|---------|---------|
| Wildcard | `_` | anything (discards) |
| Variable | `x` | anything, binds to `x` |
| Literal | `42`, `"hi"`, `true` | exact value |
| Enum variant (unit) | `None` | that variant |
| Enum variant (tuple) | `Some(x)` | variant with data, binds payload |
| Enum variant (struct) | `Point { x, y }` | struct variant, binds fields |
| Tuple | `(a, b)` | two-element tuple |
| Or | `1 \| 2 \| 3` | any of the alternatives |

Patterns also appear in `let` (destructuring) and `for` (iteration target).

---

## 9. Error Handling

> 📘 **Canonical reference**: [error-handling.md](error-handling.md) · [spec.md §3.9](spec.md#39-try-operator) · [spec.md §9.10–9.11](spec.md#910-option-prelude)

Arukellt uses `Result<T, E>` for recoverable errors and `Option<T>` for nullable values.
There is no exception mechanism.

### Result

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
use std::prelude::{parse_i32}

fn parse_positive(s: String) -> Result<i32, String> {
    let n = parse_i32(s)?
    if n < 0 {
        return Err(String_from("expected positive number"))
    }
    Ok(n)
}
```

Consume the result:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio

match parse_positive(String_from("42")) {
    Ok(n)  => stdio::println((n).to_string()),
    Err(e) => stdio::println(e),
}
```

### Option

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio

match v.get(0) {
    Some(val) => stdio::println((val).to_string()),
    None      => stdio::println(String_from("empty")),
}
```

### The `?` operator

`expr?` early-returns a failure variant from the enclosing function:

- `Result` — propagate `Err`, converting via `From` when the error types differ
- `Option` — propagate `None` (no conversion)

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
use std::prelude::{parse_i32}

fn double_parse(s: String) -> Result<i32, String> {
    let n = parse_i32(s)?   // returns Err early if parsing fails
    Ok(n * 2)
}
```

### The `Error` trait

`std::error::Error` is a provisional, statically dispatched protocol for error
payloads that also implement `Display`. `source()` currently returns an
`Option<String>` rendered immediate cause; it does not create a `dyn Error`
chain. `IoError` and `String` implement the trait in the standard library.

For an application boundary, define an enum such as `AppError`, add
`From<IoError>` and `From<String>` implementations, and let `?` convert into
the application error type. See
[`question_mark/error_ecosystem.ark`](../../tests/fixtures/question_mark/error_ecosystem.ark)
for the executable cross-library example.

### panic

For unrecoverable situations (programming errors, assertion failures):

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
Std::panic(String_from("unreachable state"))
```

In the current implementation `panic` writes to stderr and traps. It is not intended
for ordinary control flow.

---

## 10. Collections

### Vec

Dynamic arrays. Mutating a `Vec` requires a `let mut` binding:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let mut v: Vec<i32> = Vec::new<i32>()
v.push(1)
v.push(2)
let n = v.len()             // 2
let first = v.get(0)        // Some(1)
```

Common operations:

| Method | Description |
|--------|-------------|
| `Vec::new<T>() -> Vec<T>` | Empty `Vec<T>` |
| `v.push(x)` | Append element |
| `v.len() -> i32` | Length |
| `v.get(i) -> Option<T>` | Get by index |
| `v.set(i, x)` | Set by index |
| `v.pop() -> Option<T>` | Remove and return last |
| `v.iter().map(f).collect() -> Vec<U>` | Transform elements |
| `v.iter().filter(f).collect() -> Vec<T>` | Select elements |
| `v.iter().fold(init, f) -> T` | Reduce to single value |

### Array

Fixed-size arrays (stack-allocated value type):

<!-- fixture: arrays/array_literal.ark -->
<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let arr: [i32; 3] = [1, 2, 3]
let repeated: [i32; 5] = [0; 5]
let second = arr[1]   // 2
```

### HashMap

Generic hash map (ADR-066). The concrete key/value pairs supported today
are listed in [spec.md §9.14](spec.md#914-hashmap-generic-public-api).
The current compiler resolves the `HashMap` name with no import:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let mut m = HashMap::new<i32, i32>()
let _ = m.insert(1, 100)
let val = m.get(1)   // Some(100)
```

---

## 11. Closures

Closures capture variables from the surrounding scope:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::core::iter

let threshold = 5
let above = numbers.iter().filter(|x| x > threshold).collect()
```

With type annotations:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let add = |a: i32, b: i32| -> i32 { a + b }
let result = add(3, 4)   // 7
```

---

## 12. Generics

> 📘 **Canonical reference**: [spec.md §2.7 Generics](spec.md#27-generics) · [type-system.md](type-system.md)

Functions, structs, and enums can be parameterised by type variables:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
fn first<T>(v: Vec<T>) -> Option<T> {
    v.get(0)
}

struct Wrapper<T> {
    value: T,
}

enum Either<L, R> {
    Left(L),
    Right(R),
}
```

Generics are compiled via monomorphisation — each concrete type instantiation generates
specialised code. See [spec.md §2.7](spec.md#27-generics) for the normative definition.

> **Note**: Traits and `impl` blocks (`trait Foo { … }` / `impl Foo for Bar { … }`) are
> a v1 feature marked **provisional** and not covered in this guide. See
> [spec.md §2.9](spec.md#29-traits-and-impl-blocks) once you are ready to explore them.

---

## 13. Imports and Modules

> 📘 **Canonical reference**: [spec.md §7 Module System](spec.md#7-module-system)

Each `.ark` file is a module. `use` is the canonical import form
(ADR-009 / ADR-031) — for sibling files and stdlib modules alike:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use math
use utils as u
use std::host::stdio

let result = math::add(1, 2)
let val = u::compute(10)
stdio::println(String_from("from stdio"))
```

The legacy `import` spelling is a migration form that resolves the same way
but emits warning `W0101`; write `use` in new code:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
import math        // W0101: prefer `use math`
import utils as u  // W0101: prefer `use utils as u`
```

> **Note**: The module system (visibility rules, `use` resolution) carries a
> **provisional** stability label. See [spec.md §7](spec.md#7-module-system) for details.

---

## 14. Standard Library Quick Reference

The prelude stable set is available in every module without import;
all other prelude decls require an explicit `use std::prelude::{name}`:

### Strings

<!-- fixture: guide/string_ops.ark -->
<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let s = String_from("hello")
let t = s.concat(String_from(" world"))
let upper = s.to_upper()
let lower = s.to_lower()
let n = s.len()
let b = s.eq(String_from("hello"))   // true
let sub = s.slice(0, 3)              // "hel"
let trimmed = String_from("  hi  ").trim()
```

String values are valid UTF-8. The byte APIs and Unicode scalar APIs use different
index domains:

- byte_at and byte_len inspect UTF-8 bytes.
- `s.len_chars()` and `s.codepoint_at(i)` use Unicode scalar indices.
- `s.slice_bytes(start, end)` requires UTF-8 boundaries;
  `s.slice_chars(start, end)` uses scalar indices.
- `s.chars()` returns one String per scalar value. Combining marks are
  separate scalars; grapheme clusters and normalization are not implicit.

`String::try_from_utf8(bytes)` rejects invalid input.
`String::from_utf8_lossy(bytes)` replaces each invalid sequence with U+FFFD.
These constructors replace the deprecated `std::text::try_from_utf8` /
`std::text::from_utf8` module frees. char_to_string and push_char encode a char as canonical
UTF-8. The deprecated char_at name remains a raw byte compatibility API; new code
should use byte_at or codepoint_at according to its intent.

### Conversions

#### Stringification — `x.to_string()`

`x.to_string()` is the **canonical surface** for converting any scalar value to a `String`.
It accepts `i32`, `i64`, `f64`, `bool`, and `String` (pass-through).
The free-function form `to_string(x)` is a compatibility spelling pending #917.

<!-- fixture: guide/to_string_method.ark -->
<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
use std::host::stdio

fn main() {
    stdio::println((42).to_string())    // "42"
    stdio::println((0).to_string())     // "0"
    stdio::println((-7).to_string())    // "-7"
}
```

Works the same for other scalar types:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
(3.14).to_string()    // f64  → "3.14"
true.to_string()      // bool → "true"
false.to_string()     // bool → "false"
(9001_i64).to_string()  // i64 → "9001"
```

Combine with `concat` to build strings from mixed values:

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
let msg = String_from("count: ").concat(n.to_string())
```

#### Other numeric conversions

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::core::convert

let s = (42).to_string()        // i32 → String
let n = i32::try_from(s)?       // String → Result<i32, String>
let f = f64::try_from(s)?       // String → Result<f64, String>
let wide: i64 = (42).into()     // i32 → i64
let narrow = wide.to_i32_checked().unwrap_or(0)
```

### Math

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::prelude::{abs, max, min, sqrt}

let abs_val = abs(x)
let sq = sqrt(2.0)
let m = max(a, b)
let mn = min(a, b)
let p = (2).pow(10)         // i32 method
```

### I/O

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::stdio

stdio::println(String_from("line"))    // write a line
stdio::print(String_from("no newline"))
```

### Assertions

<!-- skip-doc-check reason="doc example not fixture-backed yet" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::test::Test

assert(x > 0)
Test::assert_eq_i32(actual, expected)
```

---

## Next Steps

> **Recommended next**: [type-system.md](type-system.md) — continue the beginner path with types, generics, and inference.

After the type system, follow the full reading order in [README.md](README.md#reading-paths):

| Step | Document | Topic |
|------|----------|-------|
| ✅ 1 | guide.md | You are here |
| → 2 | [type-system.md](type-system.md) | Types, generics, and inference |
| 3 | [error-handling.md](error-handling.md) | Result, Option, error propagation |
| 4 | [memory-model.md](memory-model.md) | GC-native ownership and lifetimes |
| 5 | [syntax.md](syntax.md) | Complete syntax reference |

**Reference material** (read any time):

- **Normative specification**: [spec.md](spec.md) — authoritative spec including provisional features
- **Feature stability**: [maturity-matrix.md](maturity-matrix.md) — what's stable, provisional, or experimental
- **Standard library**: [../stdlib/README.md](../stdlib/README.md)
