# Arukellt Language Specification

> **Status**: Frozen for v5 (self-hosting).
> Post-freeze changes require an ADR.

This document is the authoritative reference for the Arukellt programming
language as implemented on the `feature-arukellt-v1` branch. It is intended
to contain enough detail to re-implement the parser, type checker, and code
generator from scratch.

---

## Table of Contents

1. [Lexical Structure](#1-lexical-structure)
2. [Type System](#2-type-system)
3. [Expressions](#3-expressions)
4. [Statements](#4-statements)
5. [Pattern Matching](#5-pattern-matching)
6. [Items](#6-items)
7. [Module System](#7-module-system)
8. [Operator Precedence](#8-operator-precedence)
9. [Standard Library API](#9-standard-library-api)
10. [Error Codes](#10-error-codes)

---

## 1. Lexical Structure

### 1.1 Source Encoding

Source files are UTF-8 encoded. No BOM is required.

### 1.2 Comments

| Syntax | Description |
|--------|-------------|
| `// …` | Line comment (to end of line) |
| `/* … */` | Block comment (nestable) |
| `/// …` | Outer doc comment (attached to next item) |
| `//! …` | Inner doc comment (attached to enclosing module) |

### 1.3 Keywords

**Active (v0)**

```
fn   struct   enum   let   mut   if   else   match
while   loop   for   in   break   continue   return
pub   import   as
```

**Active (v1)**

```
trait   impl
```

**Active (v3)**

```
use
```

**Reserved (future)**

```
async   await   dyn   where   type   const   unsafe
extern   mod   super   Self
```

### 1.4 Identifiers

```
ident = [a-zA-Z_][a-zA-Z0-9_]*
```

Identifiers beginning with `__intrinsic_` are reserved for compiler
intrinsics and must not appear in user code.

### 1.5 Literals

#### Integer Literals

```
decimal     = [0-9][0-9_]*
hexadecimal = 0x[0-9a-fA-F][0-9a-fA-F_]*
binary      = 0b[01][01_]*
```

Optional type suffix: `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `i8`, `i16`.

Without a suffix, the default type is `i32`.

#### Float Literals

```
float = [0-9]+.[0-9]+ ([eE][+-]?[0-9]+)?
```

Optional type suffix: `f32`, `f64`.

Without a suffix, the default type is `f64`.

#### String Literals

Double-quoted: `"hello\nworld"`.

Standard escape sequences: `\\`, `\"`, `\n`, `\r`, `\t`, `\0`.

#### F-String Literals (Interpolated Strings)

```
f"hello {expr}"
```

Desugared by the compiler to `concat(…, to_string(expr), …)`.

#### Character Literals

Single-quoted: `'a'`, `'\n'`.

#### Boolean Literals

`true`, `false`.

### 1.6 Operators and Punctuation

**Arithmetic**: `+`, `-`, `*`, `/`, `%`

**Comparison**: `==`, `!=`, `<`, `<=`, `>`, `>=`

**Logical**: `&&`, `||`, `!`

**Bitwise**: `&`, `|`, `^`, `~`, `<<`, `>>`

**Assignment**: `=`

**Delimiters**: `(`, `)`, `{`, `}`, `[`, `]`

**Separators**: `,`, `;`, `.`, `..`, `->`, `=>`, `?`, `:`, `::`

### 1.7 Whitespace and Newlines

Whitespace (spaces, tabs) is not significant and is consumed between
tokens. Newlines are lexed as `Newline` tokens but are generally
discarded by the parser except as optional statement terminators.
Semicolons are optional.

---

## 2. Type System

### 2.1 Primitive Types

| Type | Description | Wasm Mapping |
|------|-------------|--------------|
| `i32` | 32-bit signed integer | `i32` |
| `i64` | 64-bit signed integer | `i64` |
| `i8` | 8-bit signed integer | `i32` (sign-extended) |
| `i16` | 16-bit signed integer | `i32` (sign-extended) |
| `u8` | 8-bit unsigned integer | `i32` (masked) |
| `u16` | 16-bit unsigned integer | `i32` (masked) |
| `u32` | 32-bit unsigned integer | `i32` |
| `u64` | 64-bit unsigned integer | `i64` |
| `f32` | 32-bit floating point | `f32` |
| `f64` | 64-bit floating point | `f64` |
| `bool` | Boolean (`true` / `false`) | `i32` |
| `char` | Unicode scalar value | `i32` |
| `()` | Unit type | (none) |
| `String` | UTF-8 string (reference type) | GC-managed ref |

### 2.2 Composite Types

| Type | Syntax | Semantics |
|------|--------|-----------|
| Struct | `struct Name { field: T, … }` | Named product type (reference) |
| Enum | `enum Name { Variant, Variant(T), … }` | Tagged union (reference) |
| Tuple | `(T1, T2, …)` | Anonymous product type (value) |
| Array | `[T; N]` | Fixed-size homogeneous sequence (value) |
| Slice | `[T]` | Dynamically-sized view |
| `Vec<T>` | Generic resizable vector | Reference type |
| `Option<T>` | `None \| Some(T)` | Built-in enum |
| `Result<T, E>` | `Ok(T) \| Err(E)` | Built-in enum |
| `HashMap<K, V>` | Hash map (monomorphic variants only) | Reference type |
| `Box<T>` | Heap-allocated wrapper | Reference type |
| Function | `fn(T1, T2) -> R` | First-class function type |

### 2.3 Special Types

| Type | Description |
|------|-------------|
| `Never` | Type of diverging expressions (`return`, `panic`, `break`). Compatible with every type. |
| `Error` | Sentinel used during error recovery in the type checker. |
| `Any` | Polymorphic type (erased to `anyref` at Wasm level). Used only by `to_string`. |

### 2.4 Value vs Reference Semantics

**Value types** are copied on assignment:
`i32`, `i64`, `f32`, `f64`, `u8`, `u16`, `u32`, `u64`, `i8`, `i16`,
`bool`, `char`, `()`, tuples, arrays.

**Reference types** are GC-managed heap pointers:
`String`, `Vec<T>`, `Option<T>`, `Result<T, E>`, structs, enums,
`Box<T>`, slices, functions.

### 2.5 Type Inference

- Integer literals without suffix default to `i32`.
- Float literals without suffix default to `f64`.
- `let` bindings infer their type from the initialiser expression.
- Empty collections require explicit type annotation or a typed
  constructor (e.g., `Vec_new_i32()`).
- Functions with no declared return type default to `()`.

### 2.6 Type Coercion (Numeric Promotion)

Implicit widening occurs in binary operations when operand types differ:

| Left | Right | Result |
|------|-------|--------|
| `i32` | `i64` | `i64` |
| `i64` | `i32` | `i64` |
| `i32` | `f64` | `f64` |
| `f64` | `i32` | `f64` |
| `i64` | `f64` | `f64` |
| `f64` | `i64` | `f64` |

**No implicit narrowing** is ever performed. Narrowing requires an
explicit conversion.

### 2.7 Generics

Functions, structs, and enums may be parameterised by type variables:

```ark
fn identity<T>(x: T) -> T { x }
struct Pair<A, B> { first: A, second: B }
enum Either<L, R> { Left(L), Right(R) }
```

Type parameter bounds are supported (v1):

```ark
fn print_value<T: Display>(x: T) { … }
```

The issue specification states a maximum of 2 type parameters per
generic item. In practice the parser accepts an arbitrary number;
the limit is a design guideline per issue #159.

Generics are compiled via **monomorphisation** — specialised code is
generated for each concrete type instantiation (ADR-003).

### 2.8 Traits and Impl Blocks (v1)

```ark
trait Display {
    fn display(self) -> String
}

impl Display for Point {
    fn display(self) -> String { … }
}
```

- Traits are **not** available in v0; they are a v1 feature.
- Static dispatch only (no `dyn`).
- Built-in traits planned: `Display`, `Eq`, `Hash`, `Into`/`From` (ADR-004).
- Operator overloading via magic method names: `__add`, `__sub`, `__mul`,
  `__div`, `__rem`, `__eq`, `__cmp`. Methods are mangled as
  `StructName__methodname`.

---

## 3. Expressions

### 3.1 Literal Expressions

```
42          // i32
42i64       // i64
3.14        // f64
3.14f32     // f32
"hello"     // String
f"x = {x}" // interpolated String
'a'         // char
true        // bool
false       // bool
```

### 3.2 Identifier and Qualified Identifier

```
x                // local variable or function
Module::name     // qualified name (enum variant, module item)
```

### 3.3 Binary Operations

```
left op right
```

See §8 for the full operator precedence table.

Arithmetic: `+`, `-`, `*`, `/`, `%`
Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
Logical: `&&`, `||`
Bitwise: `&`, `|`, `^`, `<<`, `>>`

Both operands of arithmetic and bitwise operators must have the same
numeric type (after coercion per §2.6). The result type is the same as
the operands.

Comparison operators return `bool`. Logical operators require `bool`
operands and return `bool`.

### 3.4 Unary Operations

| Operator | Operand | Result |
|----------|---------|--------|
| `-` (Neg) | numeric | same as operand |
| `!` (Not) | `bool` | `bool` |
| `~` (BitNot) | integer | same as operand |

### 3.5 Function Call

```
name(arg1, arg2, …)
name::<T>(arg1, …)      // with explicit type arguments
```

### 3.6 Method Call

```
expr.method(arg1, …)
```

Resolved via `impl` blocks. The receiver is the first parameter.
Methods are mangled as `TypeName__method_name` internally.

### 3.7 Field Access

```
expr.field
```

### 3.8 Index Access

```
expr[index]
```

### 3.9 Try Operator

```
expr?
```

Propagates the `Err` variant of a `Result`. The enclosing function must
return `Result<_, E>`.

### 3.10 If Expression

```ark
if cond {
    expr1
} else {
    expr2
}
```

When used as an expression (i.e., its value is consumed), both branches
must have the same type. The `else` branch may be omitted when used as
a statement, in which case the type is `()`.

`else if` chains are permitted:

```ark
if a { … } else if b { … } else { … }
```

### 3.11 Match Expression

```ark
match scrutinee {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```

All arms must produce the same type. Guards are optional boolean
expressions.

### 3.12 Block Expression

```ark
{
    stmt1;
    stmt2;
    tail_expr
}
```

The value of a block is its **tail expression** — the last expression
without a trailing semicolon. If the last statement ends with `;` or the
block is empty, the block's type is `()`.

### 3.13 Struct Literal

```ark
Point { x: 1, y: 2 }
Point { x: 1, ..base }    // struct update syntax
```

### 3.14 Tuple Expression

```ark
(1, "hello", true)
```

### 3.15 Array Expressions

```ark
[1, 2, 3]        // array literal
[0; 10]           // array repeat: 10 elements, each 0
```

### 3.16 Closure Expression

```ark
|x| x + 1
|x: i32, y: i32| -> i32 { x + y }
```

Closures capture variables from the enclosing scope. Parameter types
may be annotated or inferred.

### 3.17 Loop Expression

```ark
loop {
    if done { break value }
}
```

`loop` produces a value via `break value`.

### 3.18 Return and Control Flow

```ark
return expr     // early return from function
return          // return ()
break           // exit loop
break expr      // exit loop with value
continue        // skip to next loop iteration
```

### 3.19 Assignment

```ark
x = expr
obj.field = expr
v[i] = expr
```

Assignment target must be a mutable variable, field, or index
expression. Assignment returns `()`.

---

## 4. Statements

### 4.1 Let Binding

```ark
let name = expr
let name: Type = expr
let mut name = expr
let mut name: Type = expr
```

Destructuring via pattern:

```ark
let (a, b) = (1, 2)
```

The `mut` qualifier allows the binding to be reassigned.

### 4.2 Expression Statement

```ark
expr;
expr      // semicolon optional
```

### 4.3 While Loop

```ark
while cond {
    body
}
```

### 4.4 Loop

```ark
loop {
    body
}
```

### 4.5 For Loop

Three forms of iteration:

```ark
for i in start..end {     // range: [start, end)
    body
}

for item in values(v) {   // values iterator
    body
}

for item in expr {         // generic iterator
    body
}
```

### 4.6 Semicolons

Semicolons are optional statement terminators. Within a block, the
absence of a semicolon on the last expression makes it the block's
tail expression (return value).

---

## 5. Pattern Matching

Patterns appear in `match` arms, `let` bindings, and `for` targets.

### 5.1 Wildcard

```ark
_ => …
```

Matches anything, discards the value.

### 5.2 Variable Binding

```ark
x => …
```

Binds the matched value to `x`.

### 5.3 Literal Patterns

```ark
42 => …
"hello" => …
true => …
'a' => …
```

### 5.4 Enum Variant Pattern

```ark
Some(x) => …
Ok(value) => …
Shape::Circle(r) => …
```

### 5.5 Struct Pattern

```ark
Point { x, y } => …
Point { x: px, y: py } => …
```

### 5.6 Tuple Pattern

```ark
(a, b, c) => …
```

### 5.7 Or Pattern

```ark
1 | 2 | 3 => …
Some(0) | None => …
```

### 5.8 Match Guards

```ark
x if x > 0 => …
```

---

## 6. Items

Items are top-level declarations within a module.

### 6.1 Function Definition

```ark
fn name(param1: T1, param2: T2) -> RetType {
    body
}

pub fn name<T>(param: T) -> T {
    body
}
```

- `pub` makes the function visible to other modules.
- Return type defaults to `()` if omitted.

### 6.2 Struct Definition

```ark
struct Name {
    field1: Type1,
    field2: Type2,
}

pub struct Name<T> {
    data: T,
}
```

### 6.3 Enum Definition

```ark
enum Name {
    Unit,
    Tuple(Type1, Type2),
    Struct { field: Type },
}

pub enum Name<T> {
    None,
    Some(T),
}
```

Variant forms:
- **Unit**: `Variant` — no data.
- **Tuple**: `Variant(T1, T2, …)` — positional fields.
- **Struct**: `Variant { field: T, … }` — named fields.

### 6.4 Trait Definition (v1)

```ark
trait Name {
    fn method(param: T) -> R
}

pub trait Name<T> {
    fn method(self, x: T) -> T
}
```

### 6.5 Impl Block (v1)

```ark
impl Name {
    fn method(self) -> R { … }
}

impl Trait for Name {
    fn method(self) -> R { … }
}
```

---

## 7. Module System

### 7.1 Import Syntax

Two keywords are used for different scopes:

**`import`** — file-level module import (v0):

```ark
import math
import utils as u
```

`import` loads a sibling `.ark` file. Qualified access via `math::add(…)`.
Aliases rename the module locally.

**`use`** — path-based import (v3):

```ark
use std::host::stdio
```

`use` resolves a `::` separated path within the standard library.
Qualified access via `stdio::println(…)`.

Per ADR-009, `import` is reserved for future Component Model / WIT
boundary imports (v4+). Source module imports should prefer `use`.

### 7.2 Module Structure

Each `.ark` file is a module. A module consists of:

1. `//!` inner doc comments
2. Import / use declarations
3. Item definitions (`fn`, `struct`, `enum`, `trait`, `impl`)

### 7.3 Visibility

- Items are **private** by default (visible only within the defining module).
- The `pub` keyword makes an item visible to importing modules.

### 7.4 Prelude

The following types and values are injected into every module without
an explicit import:

**Types**: `Option`, `Result`, `String`, `Vec`

**Values**: `Some`, `None`, `Ok`, `Err`, `true`, `false`

**Functions**: All functions listed as `prelude = true` in
`std/manifest.toml` (see §9).

---

## 8. Operator Precedence

From highest to lowest. All binary operators are **left-associative**
unless noted.

| Precedence | Operators | Category |
|:----------:|-----------|----------|
| 12 | `.` `()` `[]` `?` | Postfix |
| 11 | `-` (unary) `!` `~` | Unary (right-assoc) |
| 10 | `*` `/` `%` | Multiplicative |
| 9 | `+` `-` | Additive |
| 8 | `<<` `>>` | Shift |
| 7 | `<` `<=` `>` `>=` | Comparison |
| 6 | `==` `!=` | Equality |
| 5 | `&` | Bitwise AND |
| 4 | `^` | Bitwise XOR |
| 3 | `\|` | Bitwise OR |
| 2 | `&&` | Logical AND |
| 1 | `\|\|` | Logical OR |
| 0 | `=` | Assignment (right-assoc) |

---

## 9. Standard Library API

Authoritative source: `std/manifest.toml` (263 entries).

Functions marked **prelude** are available in every module without an
import. Non-prelude functions require a `use` or `import` declaration.

### 9.1 String — Construction & Operations (prelude)

| Signature | Description |
|-----------|-------------|
| `String_from(s: String) -> String` | Construct from literal |
| `String_new() -> String` | Empty string |
| `eq(a: String, b: String) -> bool` | Equality |
| `concat(a: String, b: String) -> String` | Concatenation |
| `clone(s: String) -> String` | Clone |
| `starts_with(s: String, prefix: String) -> bool` | Prefix test |
| `ends_with(s: String, suffix: String) -> bool` | Suffix test |
| `to_lower(s: String) -> String` | Lowercase |
| `to_upper(s: String) -> String` | Uppercase |
| `slice(s: String, start: i32, end: i32) -> String` | Substring by index |
| `trim(s: String) -> String` | Strip leading/trailing whitespace |
| `contains(s: String, sub: String) -> bool` | Substring test |
| `char_at(s: String, i: i32) -> i32` | Code point at index |
| `substring(s: String, start: i32, end: i32) -> String` | Substring |
| `replace(s: String, from: String, to: String) -> String` | Replace all |
| `split(s: String, delim: String) -> Vec<String>` | Split by delimiter |
| `join(v: Vec<String>, sep: String) -> String` | Join with separator |
| `push_char(s: String, c: char)` | Append character (mutates) |
| `is_empty(s: String) -> bool` | Empty test |

### 9.2 Type Conversions (prelude)

| Signature | Description |
|-----------|-------------|
| `i32_to_string(n: i32) -> String` | i32 → String |
| `i64_to_string(n: i64) -> String` | i64 → String |
| `f64_to_string(n: f64) -> String` | f64 → String |
| `bool_to_string(b: bool) -> String` | bool → String |
| `char_to_string(c: char) -> String` | char → String |
| `to_string(x: any) -> String` | Canonical polymorphic conversion |
| `parse_i32(s: String) -> Result<i32, String>` | String → i32 |

`to_string(x)` is the canonical user-facing conversion form. Type-specific helpers such as `i32_to_string` and `char_to_string` remain available as compatibility and backend-mapped surface.
| `parse_i64(s: String) -> Result<i64, String>` | String → i64 |
| `parse_f64(s: String) -> Result<f64, String>` | String → f64 |

Non-prelude: `f32_to_string(f: f32) -> String`.

### 9.3 Math (prelude)

| Signature | Description |
|-----------|-------------|
| `sqrt(x: f64) -> f64` | Square root |
| `abs(x: i32) -> i32` | Absolute value |
| `min(a: i32, b: i32) -> i32` | Minimum |
| `max(a: i32, b: i32) -> i32` | Maximum |
| `clamp_i32(x: i32, lo: i32, hi: i32) -> i32` | Clamp to range |

### 9.4 Control (prelude)

| Signature | Description |
|-----------|-------------|
| `panic(s: String)` | Abort with message |

### 9.5 Vec — Constructors (prelude)

| Signature | Description |
|-----------|-------------|
| `Vec_new_i32() -> Vec<i32>` | Empty i32 vector |
| `Vec_new_i64() -> Vec<i64>` | Empty i64 vector |
| `Vec_new_f64() -> Vec<f64>` | Empty f64 vector |
| `Vec_new_String() -> Vec<String>` | Empty String vector |
| `Vec_new_i32_with_cap(cap: i32) -> Vec<i32>` | Pre-allocated i32 vector |
| `Vec_new_i64_with_cap(cap: i32) -> Vec<i64>` | Pre-allocated i64 vector |
| `Vec_new_f64_with_cap(cap: i32) -> Vec<f64>` | Pre-allocated f64 vector |
| `Vec_with_capacity_i32(cap: i32) -> Vec<i32>` | Alias for Vec_new_i32_with_cap |
| `Vec_with_capacity_String(cap: i32) -> Vec<String>` | Pre-allocated String vector |

### 9.6 Vec — Operations (prelude)

| Signature | Description |
|-----------|-------------|
| `push(v: Vec<T>, x: T)` | Append element |
| `pop(v: Vec<T>) -> Option<T>` | Remove and return last |
| `len(v: Vec<T>) -> i32` | Length |
| `get(v: Vec<T>, i: i32) -> Option<T>` | Bounds-checked access |
| `get_unchecked(v: Vec<T>, i: i32) -> T` | Unchecked access |
| `set(v: Vec<T>, i: i32, x: T)` | Set element at index |
| `clear(v: Vec<T>)` | Remove all elements |
| `as_slice(v: Vec<T>) -> Vec<T>` | View as slice |

### 9.7 Vec — Sorting (prelude)

| Signature | Description |
|-----------|-------------|
| `sort_i32(v: Vec<i32>)` | In-place sort (bubble sort) |
| `sort_i64(v: Vec<i64>)` | In-place sort |
| `sort_f64(v: Vec<f64>)` | In-place sort |
| `sort_String(v: Vec<String>)` | In-place sort |

### 9.8 Vec — Higher-Order Functions (prelude)

| Signature | Description |
|-----------|-------------|
| `map_i32_i32(v: Vec<i32>, f: fn(i32) -> i32) -> Vec<i32>` | Map |
| `map_i64_i64(v: Vec<i64>, f: fn(i64) -> i64) -> Vec<i64>` | Map |
| `map_f64_f64(v: Vec<f64>, f: fn(f64) -> f64) -> Vec<f64>` | Map |
| `map_String_String(v: Vec<String>, f: fn(String) -> String) -> Vec<String>` | Map |
| `filter_i32(v: Vec<i32>, f: fn(i32) -> bool) -> Vec<i32>` | Filter |
| `filter_i64(v: Vec<i64>, f: fn(i64) -> bool) -> Vec<i64>` | Filter |
| `filter_f64(v: Vec<f64>, f: fn(f64) -> bool) -> Vec<f64>` | Filter |
| `filter_String(v: Vec<String>, f: fn(String) -> bool) -> Vec<String>` | Filter |
| `fold_i32_i32(v: Vec<i32>, init: i32, f: fn(i32, i32) -> i32) -> i32` | Fold |
| `fold_i64_i64(v: Vec<i64>, init: i64, f: fn(i64, i64) -> i64) -> i64` | Fold |
| `any_i32(v: Vec<i32>, f: fn(i32) -> bool) -> bool` | Any match |
| `find_i32(v: Vec<i32>, f: fn(i32) -> bool) -> Option<i32>` | Find first |

### 9.9 Vec — Aggregation & Mutation (prelude)

| Signature | Description |
|-----------|-------------|
| `sum_i32(v: Vec<i32>) -> i32` | Sum |
| `product_i32(v: Vec<i32>) -> i32` | Product |
| `contains_i32(v: Vec<i32>, x: i32) -> bool` | Membership test |
| `contains_String(v: Vec<String>, x: String) -> bool` | Membership test |
| `reverse_i32(v: Vec<i32>)` | In-place reverse |
| `reverse_String(v: Vec<String>)` | In-place reverse |
| `remove_i32(v: Vec<i32>, index: i32)` | Remove at index |

### 9.10 Option (prelude)

| Signature | Description |
|-----------|-------------|
| `unwrap(o: Option<T>) -> T` | Extract or panic |
| `unwrap_or(o: Option<T>, default: T) -> T` | Extract or default |
| `unwrap_or_else(o: Option<T>, f: fn() -> T) -> T` | Extract or compute |
| `is_some(o: Option<T>) -> bool` | Some test |
| `is_none(o: Option<T>) -> bool` | None test |
| `expect(o: Option<T>, msg: String) -> T` | Extract or panic with message |
| `map_option_i32_i32(o: Option<i32>, f: fn(i32) -> i32) -> Option<i32>` | Map over Option |
| `map_option_String_String(o: Option<String>, f: fn(String) -> String) -> Option<String>` | Map over Option |

### 9.11 Result (prelude)

| Signature | Description |
|-----------|-------------|
| `is_ok(r: Result<T, E>) -> bool` | Ok test |
| `is_err(r: Result<T, E>) -> bool` | Err test |
| `ok(r: Result<T, E>) -> Option<T>` | Convert to Option |
| `err(r: Result<T, E>) -> Option<E>` | Extract error |
| `ok_or(o: Option<T>, e: E) -> Result<T, E>` | Option → Result |
| `map_result_i32_i32(r: Result<i32, E>, f: fn(i32) -> i32) -> Result<i32, E>` | Map over Result |

### 9.12 Box (prelude)

| Signature | Description |
|-----------|-------------|
| `Box_new(x: T) -> Box<T>` | Wrap in box |
| `unbox(b: Box<T>) -> T` | Unwrap |

### 9.13 Assertions (prelude)

| Signature | Description |
|-----------|-------------|
| `assert(cond: bool)` | Assert true |
| `assert_eq(a: i32, b: i32)` | Assert equal (i32) |
| `assert_ne(a: i32, b: i32)` | Assert not equal (i32) |
| `assert_eq_i64(a: i64, b: i64)` | Assert equal (i64) |
| `assert_eq_str(a: String, b: String)` | Assert equal (String) |

### 9.14 HashMap (prelude, monomorphic)

| Signature | Description |
|-----------|-------------|
| `HashMap_i32_i32_new() -> HashMap<i32, i32>` | New map |
| `HashMap_i32_i32_insert(m: HashMap<i32, i32>, k: i32, v: i32)` | Insert |
| `HashMap_i32_i32_get(m: HashMap<i32, i32>, k: i32) -> Option<i32>` | Lookup |
| `HashMap_i32_i32_contains_key(m: HashMap<i32, i32>, k: i32) -> bool` | Key test |
| `HashMap_i32_i32_len(m: HashMap<i32, i32>) -> i32` | Size |

### 9.15 Host — Standard I/O (`std::host::stdio`)

| Signature | Description |
|-----------|-------------|
| `print(s: String)` | Print without newline |
| `println(s: String)` | Print with newline |
| `eprintln(s: String)` | Print to stderr |

### 9.16 Host — File System (`std::host::fs`)

| Signature | Description |
|-----------|-------------|
| `read_to_string(path: String) -> Result<String, String>` | Read file |
| `write_string(path: String, data: String) -> Result<(), String>` | Write file |

### 9.17 Host — Environment (`std::host::env`)

| Signature | Description |
|-----------|-------------|
| `arg_count() -> i32` | Number of CLI arguments |
| `arg_at(i: i32) -> Option<String>` | Get argument by index |
| `args() -> Vec<String>` | All arguments |
| `has_flag(flag: String) -> bool` | Check for flag |
| `var(name: String) -> Option<String>` | Environment variable |

### 9.18 Host — Process (`std::host::process`)

| Signature | Description |
|-----------|-------------|
| `exit(code: i32)` | Exit with code |
| `abort()` | Abort immediately |

### 9.19 Host — Clock (`std::host::clock`)

| Signature | Description |
|-----------|-------------|
| `monotonic_now() -> i64` | Current monotonic time |

### 9.20 Host — Random (`std::host::random`)

| Signature | Description |
|-----------|-------------|
| `random_i32() -> i32` | Random i32 |
| `random_i32_range(lo: i32, hi: i32) -> i32` | Random in range |
| `random_bool() -> bool` | Random boolean |

### 9.21 Pure Library Functions (non-prelude)

These functions are available via stdlib modules, not auto-imported.

**Time/Duration**:
`duration_ms(a: i64, b: i64) -> i64`,
`duration_us(a: i64, b: i64) -> i64`,
`duration_ns(a: i64, b: i64) -> i64`

**Hash**:
`hash_i32(x: i32) -> i32`,
`hash_combine(a: i32, b: i32) -> i32`

**Seeded Random**:
`seeded_random(seed: i32) -> i32`,
`seeded_range(seed: i32, lo: i32, hi: i32) -> i32`,
`shuffle_i32(v: Vec<i32>, seed: i32) -> Vec<i32>`

**Range**:
`range_new(start: i32, end: i32) -> Range`,
`range_contains(r: Range, x: i32) -> bool`,
`range_len(r: Range) -> i32`

**Error**:
`error_message(e: Error) -> String`

**Path**:
`extension(path: String) -> String`,
`file_name(path: String) -> String`,
`parent(path: String) -> String`,
`is_absolute(path: String) -> bool`,
`with_extension(path: String, ext: String) -> String`

**Text**:
`trim_start(s: String) -> String`,
`trim_end(s: String) -> String`,
`pad_left(s: String, width: i32, fill: String) -> String`,
`pad_right(s: String, width: i32, fill: String) -> String`,
`index_of(s: String, sub: String) -> i32`,
`chars(s: String) -> Vec<String>`,
`lines(s: String) -> Vec<String>`,
`len_bytes(s: String) -> i32`,
`byte_length(s: String) -> i32`

**Formatting**:
`format_i32(n: i32) -> String`,
`format_i64(n: i64) -> String`,
`format_f64(f: f64) -> String`,
`format_bool(b: bool) -> String`

**Bytes**:
`bytes_new() -> Vec<i32>`,
`bytes_push(v: Vec<i32>, b: i32)`,
`bytes_get(v: Vec<i32>, i: i32) -> i32`,
`bytes_len(v: Vec<i32>) -> i32`,
`bytes_eq(a: Vec<i32>, b: Vec<i32>) -> bool`,
`hex_encode(v: Vec<i32>) -> String`,
`hex_decode(s: String) -> Vec<i32>`

**Endian Conversion**:
`u16_to_le_bytes(n: i32) -> Vec<i32>`,
`u16_from_le_bytes(v: Vec<i32>) -> i32`,
`u32_to_le_bytes(n: i32) -> Vec<i32>`,
`u32_from_le_bytes(v: Vec<i32>) -> i32`,
`u32_to_be_bytes(n: i32) -> Vec<i32>`,
`u32_from_be_bytes(v: Vec<i32>) -> i32`

**Linear Memory** (T1 target only):
`memory_copy(dst: i32, src: i32, len: i32)`,
`memory_fill(dst: i32, val: i32, len: i32)`,
`leb128_encode_i32(n: i32) -> Vec<i32>`,
`leb128_encode_u32(n: i32) -> Vec<i32>`

**Collections — Search & Sort**:
`binary_search(v: Vec<i32>, x: i32) -> i32`,
`unique(v: Vec<i32>) -> Vec<i32>`,
`count_eq(v: Vec<i32>, x: i32) -> i32`,
`min_i32(v: Vec<i32>) -> i32`,
`max_i32(v: Vec<i32>) -> i32`,
`seq_contains(v: Vec<i32>, x: i32) -> bool`,
`seq_reverse(v: Vec<i32>) -> Vec<i32>`

**Collections — Arena**:
`arena_new() -> Vec<i32>`,
`arena_alloc(a: Vec<i32>, size: i32) -> i32`,
`arena_get(a: Vec<i32>, idx: i32) -> i32`,
`arena_len(a: Vec<i32>) -> i32`

**Collections — Deque**:
`deque_new() -> Vec<i32>`,
`deque_push_front(d: Vec<i32>, x: i32)`,
`deque_push_back(d: Vec<i32>, x: i32)`,
`deque_pop_front(d: Vec<i32>) -> i32`,
`deque_pop_back(d: Vec<i32>) -> i32`,
`deque_len(d: Vec<i32>) -> i32`,
`deque_is_empty(d: Vec<i32>) -> bool`

**Collections — Priority Queue**:
`pq_new() -> Vec<i32>`,
`pq_push(q: Vec<i32>, x: i32)`,
`pq_pop(q: Vec<i32>) -> i32`,
`pq_peek(q: Vec<i32>) -> i32`,
`pq_len(q: Vec<i32>) -> i32`,
`pq_is_empty(q: Vec<i32>) -> bool`

**Collections — Bitset**:
`bitset_new() -> Vec<i32>`,
`bitset_mark(b: Vec<i32>, idx: i32)`,
`bitset_unmark(b: Vec<i32>, idx: i32)`,
`bitset_test(b: Vec<i32>, idx: i32) -> bool`

**Collections — Sorted Map**:
`sorted_map_new() -> Vec<i32>`,
`sorted_map_insert(m: Vec<i32>, k: i32, v: i32)`,
`sorted_map_get(m: Vec<i32>, k: i32) -> i32`,
`sorted_map_contains(m: Vec<i32>, k: i32) -> bool`,
`sorted_map_find_idx(m: Vec<i32>, k: i32) -> i32`,
`sorted_map_len(m: Vec<i32>) -> i32`

**Collections — Flat HashMap**:
`hashmap_new() -> Vec<i32>`,
`hashmap_set(m: Vec<i32>, k: i32, v: i32)`,
`hashmap_get(m: Vec<i32>, k: i32) -> i32`,
`hashmap_contains(m: Vec<i32>, k: i32) -> bool`,
`hashmap_size(m: Vec<i32>) -> i32`

**JSON (minimal)**:
`json_null() -> String`,
`json_parse_i32(s: String) -> i32`,
`json_parse_bool(s: String) -> bool`,
`json_stringify_i32(n: i32) -> String`,
`json_stringify_bool(b: bool) -> String`,
`json_stringify_string(s: String) -> String`

**CSV**:
`csv_split_line(line: String) -> Vec<String>`

**TOML**:
`toml_parse_line(line: String) -> String`

**Testing (non-prelude)**:
`assert_true(b: bool)`,
`assert_false(b: bool)`,
`assert_eq_i32(a: i32, b: i32)`,
`assert_eq_i64(a: i64, b: i64)`,
`assert_eq_f64(a: f64, b: f64)`,
`assert_eq_bool(a: bool, b: bool)`,
`assert_eq_string(a: String, b: String)`,
`assert_ne_i32(a: i32, b: i32)`,
`assert_ne_string(a: String, b: String)`,
`expect_some_i32(o: Option<i32>) -> i32`,
`expect_none_i32(o: Option<i32>)`,
`expect_ok_i32(r: Result<i32, String>) -> i32`,
`expect_err_string(r: Result<i32, String>) -> String`

**Wasm Introspection**:
`wasm_magic() -> Vec<i32>`,
`wasm_version() -> Vec<i32>`,
`section_type() -> i32`, `section_import() -> i32`,
`section_function() -> i32`, `section_table() -> i32`,
`section_memory() -> i32`, `section_global() -> i32`,
`section_export() -> i32`, `section_start() -> i32`,
`section_element() -> i32`, `section_code() -> i32`,
`section_data() -> i32`,
`valtype_i32() -> i32`, `valtype_i64() -> i32`,
`valtype_f32() -> i32`, `valtype_f64() -> i32`,
`canonical_abi_version() -> i32`,
`component_model_version() -> String`

**WIT Type Constants**:
`wit_type_u8() -> i32`, `wit_type_u16() -> i32`,
`wit_type_u32() -> i32`, `wit_type_u64() -> i32`,
`wit_type_s8() -> i32`, `wit_type_s16() -> i32`,
`wit_type_s32() -> i32`, `wit_type_s64() -> i32`,
`wit_type_f32() -> i32`, `wit_type_f64() -> i32`,
`wit_type_char() -> i32`, `wit_type_bool() -> i32`,
`wit_type_string() -> i32`,
`wit_type_name(id: i32) -> String`

---

## 10. Error Codes

The type checker emits diagnostics with the following error codes:

| Code | Description |
|------|-------------|
| E0200 | Type mismatch |
| E0202 | Undefined variable or function |
| E0205 | Incompatible types in binary operation |
| E0207 | Incorrect number of function arguments |

Additional error codes are implementation-defined and may be added
by the compiler without an ADR.

---

## Appendix A. Grammar Summary (EBNF)

```ebnf
module      = doc_comment* import* item* ;
import      = "import" IDENT ("as" IDENT)?
            | "use" IDENT ("::" IDENT)+ ("as" IDENT)? ;
item        = ("pub")? ( fn_def | struct_def | enum_def
                        | trait_def | impl_block ) ;

fn_def      = "fn" IDENT type_params? "(" param_list? ")"
              ("->" type_expr)? block ;
struct_def  = "struct" IDENT type_params? "{" field_list? "}" ;
enum_def    = "enum" IDENT type_params? "{" variant_list? "}" ;
trait_def   = "trait" IDENT type_params? "{" method_sig* "}" ;
impl_block  = "impl" (IDENT "for")? IDENT "{" fn_def* "}" ;

type_params = "<" type_param ("," type_param)* ">" ;
type_param  = IDENT (":" IDENT ("+" IDENT)*)? ;
param_list  = param ("," param)* ","? ;
param       = IDENT ":" type_expr ;
field_list  = field ("," field)* ","? ;
field       = IDENT ":" type_expr ;
variant_list= variant ("," variant)* ","? ;
variant     = IDENT
            | IDENT "(" type_expr ("," type_expr)* ")"
            | IDENT "{" field_list? "}" ;
method_sig  = IDENT "(" param_list? ")" ("->" type_expr)? ;

block       = "{" stmt* expr? "}" ;
stmt        = let_stmt | while_stmt | loop_stmt | for_stmt
            | expr_stmt ;
let_stmt    = "let" "mut"? (IDENT | pattern)
              (":" type_expr)? "=" expr ";"? ;
while_stmt  = "while" expr block ;
loop_stmt   = "loop" block ;
for_stmt    = "for" IDENT "in" for_iter block ;
for_iter    = expr ".." expr
            | "values" "(" expr ")"
            | expr ;
expr_stmt   = expr ";"? ;

expr        = assign_expr ;
assign_expr = or_expr ("=" assign_expr)? ;
or_expr     = and_expr ("||" and_expr)* ;
and_expr    = eq_expr ("&&" eq_expr)* ;
eq_expr     = cmp_expr (("==" | "!=") cmp_expr)* ;
cmp_expr    = bitor_expr (("<" | "<=" | ">" | ">=") bitor_expr)* ;
bitor_expr  = bitxor_expr ("|" bitxor_expr)* ;
bitxor_expr = bitand_expr ("^" bitand_expr)* ;
bitand_expr = shift_expr ("&" shift_expr)* ;
shift_expr  = add_expr (("<<" | ">>") add_expr)* ;
add_expr    = mul_expr (("+" | "-") mul_expr)* ;
mul_expr    = unary_expr (("*" | "/" | "%") unary_expr)* ;
unary_expr  = ("-" | "!" | "~") unary_expr | postfix_expr ;
postfix_expr= primary_expr (postfix_op)* ;
postfix_op  = "(" expr_list? ")"
            | "[" expr "]"
            | "." IDENT ("(" expr_list? ")")?
            | "?" ;

primary_expr= INT_LIT | FLOAT_LIT | STRING_LIT | CHAR_LIT
            | "true" | "false"
            | IDENT | IDENT "::" IDENT
            | "(" expr_list? ")"
            | "[" array_init "]"
            | IDENT "{" struct_fields "}"
            | "if" expr block ("else" (if_expr | block))?
            | "match" expr "{" match_arm* "}"
            | "loop" block
            | "|" closure_params? "|" ("->" type_expr)? expr
            | "return" expr?
            | "break" expr?
            | "continue" ;

array_init  = expr (";" expr)?
            | expr ("," expr)* ;
struct_fields = (IDENT ":" expr) ("," IDENT ":" expr)*
                ("," ".." expr)? ;
match_arm   = pattern ("if" expr)? "=>" expr ","? ;
closure_params = closure_param ("," closure_param)* ;
closure_param  = IDENT (":" type_expr)? ;

pattern     = "_"
            | IDENT
            | INT_LIT | FLOAT_LIT | STRING_LIT | CHAR_LIT | BOOL_LIT
            | IDENT "::" IDENT pattern_args?
            | IDENT "{" pattern_fields "}"
            | "(" pattern ("," pattern)* ")"
            | pattern ("|" pattern)+ ;
pattern_args  = "(" pattern ("," pattern)* ")" ;
pattern_fields= (IDENT (":" pattern)?) ("," IDENT (":" pattern)?)* ;

type_expr   = "()"
            | "(" type_expr ("," type_expr)+ ")"
            | "[" type_expr (";" INT_LIT)? "]"
            | "fn" "(" type_list? ")" "->" type_expr
            | IDENT
            | IDENT "." IDENT
            | IDENT "<" type_list ">" ;
type_list   = type_expr ("," type_expr)* ;
expr_list   = expr ("," expr)* ;
```

---

## Appendix B. Compilation Targets

Per ADR-007, Arukellt supports five compilation targets:

| Target | Name | Notes |
|--------|------|-------|
| T1 | `wasm32-wasi-p1` | Linear memory (no GC). AtCoder compatibility. |
| T2 | `wasm32-freestanding` | Wasm GC, Reference Types. Browser/embedded. |
| T3 | `wasm32-wasi-p2` | Wasm GC, WASI Preview 2, Component Model. Primary target. |
| T4 | `native-x86_64` | LLVM IR emission, Wasm semantics via library. |
| T5 | `native-arm64` | LLVM IR emission, Wasm semantics via library. |

---

## Appendix C. Design Decision References

| ADR | Decision |
|-----|----------|
| ADR-002 | Wasm GC adopted as primary memory model |
| ADR-003 | Limited monomorphisation for generics |
| ADR-004 | Traits deferred to v1; static dispatch only |
| ADR-005 | LLVM backend subordinate to Wasm semantics |
| ADR-006 | 3-layer public ABI (internal / Wasm / native C) |
| ADR-007 | Five compilation targets (T1–T5) |
| ADR-008 | External `wasm-tools` for Component Model wrapping |
| ADR-009 | `use` for source imports; `import` reserved for WIT (v4+) |
| ADR-011 | Host-bound APIs in `std::host::*`; pure APIs in `std::*` |
