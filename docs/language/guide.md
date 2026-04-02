# Arukellt Language Guide

> **Explanatory**: This document explains concepts and usage patterns.
> It is not the authoritative specification. For normative behavior, see [spec.md](spec.md)
> and [../current-state.md](../current-state.md).

This guide covers the **stable, implemented** features of the Arukellt programming language.
It is written for readers who want a practical introduction before diving into the full
[spec.md](spec.md). Features that are provisional, experimental, or unimplemented are
not covered here — consult the [Feature Maturity Matrix](maturity-matrix.md) for the
complete stability picture.

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

Every Arukellt program starts from a `main` function:

```ark
fn main() {
    println(String_from("Hello, world!"))
}
```

With an exit code:

```ark
fn main() -> i32 {
    println(String_from("Hello, world!"))
    0
}
```

Source files are UTF-8. Comments use `//` for line comments and `/* … */` for block comments (nestable). Doc comments use `///` on the item they annotate.

---

## 2. Variables and Bindings

Declare bindings with `let`. By default they are immutable:

```ark
let x = 42
let name = String_from("Alice")
```

Use `mut` to allow reassignment:

```ark
let mut counter = 0
counter = counter + 1
```

Type annotations are optional when the type can be inferred:

```ark
let n: i32 = 10
let s: String = String_from("hello")
```

Destructuring in `let`:

```ark
let (a, b) = (1, 2)
```

Semicolons are optional statement terminators. The last expression in a block (without a trailing `;`) is the block's return value.

---

## 3. Primitive Types

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

```ark
let x: i32 = 1
let y: i64 = 2i64
let z = x + y    // z: i64
```

No implicit narrowing is ever performed — use an explicit conversion function.

---

## 4. Functions

```ark
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

- Return type defaults to `()` when omitted.
- The last expression in the body (without `;`) is the implicit return.
- Use `return expr` for early returns.
- `pub` makes a function visible to importing modules.

Generic functions (see also §12):

```ark
fn identity<T>(x: T) -> T {
    x
}
```

---

## 5. Control Flow

### if / else

```ark
if x > 0 {
    println(String_from("positive"))
} else if x == 0 {
    println(String_from("zero"))
} else {
    println(String_from("negative"))
}
```

`if` is an expression — both branches must produce the same type when the value is used:

```ark
let label = if x > 0 { String_from("pos") } else { String_from("neg") }
```

### while

```ark
while n > 0 {
    n = n - 1
}
```

### loop

```ark
loop {
    if done { break }
}
```

`loop` can return a value via `break expr`:

```ark
let result = loop {
    if condition { break 42 }
}
```

### for

Range iteration (half-open `[start, end)`):

```ark
for i in 0..10 {
    println(to_string(i))
}
```

Iterate over a `Vec`:

```ark
for item in values(v) {
    println(to_string(item))
}
```

### break and continue

`break` exits the nearest enclosing loop. `continue` skips to the next iteration.

---

## 6. Structs

Define a named product type:

```ark
struct Point {
    x: i32,
    y: i32,
}
```

Create and access:

```ark
let p = Point { x: 3, y: 4 }
println(to_string(p.x))
```

Struct update syntax (copy all fields from `base`, override selected ones):

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

```ark
let d = Direction::North
let s = Shape::Circle(5)
```

The built-in `Option<T>` and `Result<T, E>` follow the same rules:

```ark
let maybe: Option<i32> = Some(42)
let ok: Result<i32, String> = Ok(1)
let err: Result<i32, String> = Err(String_from("bad"))
```

`Some`, `None`, `Ok`, `Err`, `Option`, `Result` are in the prelude — no import needed.

---

## 8. Pattern Matching

`match` exhaustively tests a value against a sequence of patterns:

```ark
match direction {
    Direction::North => println(String_from("north")),
    Direction::South => println(String_from("south")),
    _ => println(String_from("east or west")),
}
```

Match arms can include optional guards:

```ark
match x {
    n if n > 0 => println(String_from("positive")),
    0          => println(String_from("zero")),
    _          => println(String_from("negative")),
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

Arukellt uses `Result<T, E>` for recoverable errors and `Option<T>` for nullable values.
There is no exception mechanism.

### Result

```ark
fn parse_positive(s: String) -> Result<i32, String> {
    let n = parse_i32(s)?
    if n < 0 {
        return Err(String_from("expected positive number"))
    }
    Ok(n)
}
```

Consume the result:

```ark
match parse_positive(String_from("42")) {
    Ok(n)  => println(to_string(n)),
    Err(e) => println(e),
}
```

### Option

```ark
match vec_get(v, 0) {
    Some(val) => println(to_string(val)),
    None      => println(String_from("empty")),
}
```

### The `?` operator

`expr?` propagates the `Err` variant automatically. The enclosing function must
return `Result<_, E>` where `E` is compatible:

```ark
fn double_parse(s: String) -> Result<i32, String> {
    let n = parse_i32(s)?   // returns Err early if parsing fails
    Ok(n * 2)
}
```

### panic

For unrecoverable situations (programming errors, assertion failures):

```ark
panic(String_from("unreachable state"))
```

In the current implementation `panic` writes to stderr and traps. It is not intended
for ordinary control flow.

---

## 10. Collections

### Vec

Dynamic arrays. Use typed constructors:

```ark
let v: Vec<i32> = Vec_new_i32()
vec_push(v, 1)
vec_push(v, 2)
let len = vec_len(v)        // 2
let first = vec_get(v, 0)   // Some(1)
```

Common operations (all prelude):

| Function | Description |
|----------|-------------|
| `Vec_new_i32() -> Vec<i32>` | Empty `Vec<i32>` |
| `vec_push(v, x)` | Append element |
| `vec_len(v) -> i32` | Length |
| `vec_get(v, i) -> Option<T>` | Get by index |
| `vec_set(v, i, x)` | Set by index |
| `vec_pop(v) -> Option<T>` | Remove and return last |
| `vec_map(v, f) -> Vec<T>` | Transform elements |
| `vec_filter(v, f) -> Vec<T>` | Select elements |
| `vec_fold(v, init, f) -> T` | Reduce to single value |

### Array

Fixed-size arrays (stack-allocated value type):

```ark
let arr: [i32; 3] = [1, 2, 3]
let repeated: [i32; 5] = [0; 5]
let second = arr[1]   // 2
```

### HashMap

Monomorphic hash map (`i32 → i32` by default):

```ark
import std::collections::hash

let map = hashmap_new()
hashmap_set(map, 1, 100)
let val = hashmap_get(map, 1)   // 100
```

---

## 11. Closures

Closures capture variables from the surrounding scope:

```ark
let threshold = 5
let above = vec_filter(numbers, |x| x > threshold)
```

With type annotations:

```ark
let add = |a: i32, b: i32| -> i32 { a + b }
let result = add(3, 4)   // 7
```

---

## 12. Generics

Functions, structs, and enums can be parameterised by type variables:

```ark
fn first<T>(v: Vec<T>) -> Option<T> {
    vec_get(v, 0)
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
> [spec.md §2.8](spec.md#28-traits-and-impl-blocks-v1) once you are ready to explore them.

---

## 13. Imports and Modules

Each `.ark` file is a module. Use `import` to load a sibling file:

```ark
import math
import utils as u

let result = math::add(1, 2)
let val = u::compute(10)
```

Standard library modules use `use`:

```ark
use std::host::stdio
use std::collections::hash

stdio::println(String_from("from stdio"))
```

> **Note**: The module system (visibility rules, `use` resolution) carries a
> **provisional** stability label. The basic `import` form for sibling files
> works reliably in practice. See [spec.md §7](spec.md#7-module-system) for details.

---

## 14. Standard Library Quick Reference

The prelude (available in every module without import):

### Strings

```ark
let s = String_from("hello")
let t = concat(s, String_from(" world"))
let upper = to_upper(s)
let lower = to_lower(s)
let n = string_len(s)
let b = eq(s, String_from("hello"))   // true
let sub = slice(s, 0, 3)              // "hel"
let trimmed = trim(String_from("  hi  "))
```

### Conversions

```ark
let s = to_string(42)        // i32 → String
let n = parse_i32(s)?        // String → Result<i32, String>
let f = parse_f64(s)?        // String → Result<f64, String>
let i = f64_to_i32(3.7)      // f64 → i32 (truncates)
let b = i32_to_bool(1)
```

### Math

```ark
let abs_val = abs(x)
let sq = sqrt(2.0)
let m = max(a, b)
let mn = min(a, b)
let p = pow(2.0, 10.0)
let fl = floor(3.7)
let cl = ceil(3.2)
```

### I/O (prelude)

```ark
println(String_from("line"))    // print with newline
print(String_from("no newline"))
```

### Assertions

```ark
assert(x > 0, String_from("x must be positive"))
assert_eq(actual, expected, String_from("values differ"))
```

---

## Next Steps

- **Normative reference**: [spec.md](spec.md) — the authoritative specification of all language behavior, including provisional and planned features.
- **Type system detail**: [type-system.md](type-system.md)
- **Syntax reference**: [syntax.md](syntax.md)
- **Error handling in depth**: [error-handling.md](error-handling.md)
- **Memory model**: [memory-model.md](memory-model.md)
- **Feature stability overview**: [maturity-matrix.md](maturity-matrix.md)
- **Standard library**: [../stdlib/README.md](../stdlib/README.md)
