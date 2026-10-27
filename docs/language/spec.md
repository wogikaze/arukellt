# Arukellt Language Specification

> **Normative**: This document defines the authoritative behavior of Arukellt as implemented.
> This document is intended to describe implemented behavior. Content accuracy is manually reviewed unless a section links to a specific fixture or check ID.
> For current verified state, see [../current-state.md](../current-state.md).

This document is the authoritative reference for the Arukellt programming
language as implemented on the current mainline compiler. It is intended
to contain enough detail to re-implement the parser, type checker, and code
generator from scratch.

### Language version model

Arukellt currently has one selectable language surface: the current project
release. The CLI has no language-edition or parser-mode selector. Older labels
such as “v0”, “v1”, and “v3” in historical material name development
milestones; they are not editions that users can select and they do not define
parallel compatibility modes. This specification therefore names features by
capability rather than by those milestone numbers. Project releases use
semantic versions (`0.1.0`); Wasm ABI and host-profile versions are separate
axes documented under [Platform](../platform/README.md).

> **New to Arukellt?** Start with the [Language Guide](guide.md) for a
> practical introduction to stable, implemented features before reading this specification.

## Stability Labels

Each section and feature carries one of the following stability labels (ADR-013 §
Stability):

| Label | Meaning |
|-------|---------|
| **stable** | Feature is finalized. Breaking changes require a new major version. |
| **provisional** | Feature is implemented and tested, but the interface may change before stabilization. |
| **experimental** | Feature exists in the codebase but is not tested or guaranteed on every build. |
| **unimplemented** | Feature is specified but not yet implemented. |

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

## 1. Lexical Structure <!-- stability: stable -->

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

**Hard-reserved keywords**

```
fn   struct   enum   let   mut   if   else   match   while   loop
for   in   break   continue   return   pub   export   import   as   where
const   proof   resource   async   Self
```

`trait`, `impl`, and `use` are hard-reserved as well:

```
trait   impl   use
```

`true` and `false` are lexical boolean literals. They are not Prelude values
and cannot be introduced by a user declaration.

**Contextual keywords**

```
test   move   dyn   type   mod
```

These words remain identifiers in the lexer and are recognized only in their
grammar positions. In particular, `type` is used for associated types, `dyn`
for dynamic trait syntax, `move` for closure capture, `test` for test
declarations, and `mod` in existing module/test paths.

**Future-reserved keywords**

```
await   unsafe   extern   super
```

Future-reserved words are rejected as source identifiers even while their
syntax is unimplemented. The machine-readable list is
[`docs/data/language-keyword-policy.toml`](../data/language-keyword-policy.toml).

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

The compiler lowers each embedded expression through the typed receiver method
`expr.to_string()` (`Display`). Literal and converted segments are combined
left-to-right with the canonical `String::concat` method. The debug specifier
`{expr:?}` uses `expr.fmt_debug()` (`Debug`). These are compiler-owned method
references; the expansion does not perform ordinary source-level lookup of
`concat`, `to_string`, or `format_debug`, so a user binding with one of those
names cannot capture the generated call. Embedded expressions are evaluated
once in source order.

#### Character Literals

Single-quoted: `'a'`, `'\n'`.

#### Boolean Literals

`true`, `false`. These are `bool` literals, not Prelude names.

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
tokens. Newlines are lexed as `Newline` tokens and act as optional statement
terminators or syntax-specific continuation separators. Semicolons are
optional for ordinary block statements, but remain explicit terminators for
items and for multi-clause group boundaries.

---

## 2. Type System <!-- stability: stable -->

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

### 2.1.1 String encoding and Unicode scalar boundaries

String values are well-formed UTF-8. A char is a Unicode scalar value: code points in
the surrogate range are not valid char values. Combining marks remain independent
scalar values; the language does not define grapheme-cluster segmentation or normalization.

Byte and scalar positions are different domains. byte_at, byte_len, slice_bytes, and
to_utf8_bytes use byte indices. len_chars, codepoint_at, chars, and slice_chars use
Unicode scalar indices. A byte slice succeeds only when both bounds are within the
string and on UTF-8 scalar boundaries. The fallible forms return None; the non-fallible
slice forms return the empty string on invalid input.

try_from_utf8 is strict and returns None for an invalid byte sequence. from_utf8 is
lossy and replaces each invalid sequence with U+FFFD. char_to_string and push_char
encode valid scalar values as canonical UTF-8; an invalid internal integer is replaced
with U+FFFD. These rules are shared by wasm32-gc and wasm32.

### 2.2 Composite Types

| Type | Syntax | Semantics |
|------|--------|-----------|
| Struct | `struct Name { field: T, … }` | Named owned product type |
| Enum | `enum Name { Variant, Variant(T), … }` | Owned tagged ADT |
| Tuple | `(T1, T2, …)` | Anonymous product type (value) |
| Array | `[T; N]` | Fixed-size homogeneous sequence (value) |
| Slice | `[T]` / `Slice<T>` | Non-`Copy` read-only view over a source collection and bounded range |
| `Vec<T>` | Generic resizable vector | Reference type |
| `Option<T>` | `None \| Some(T)` | Owned tagged ADT; structurally `Copy` when `T: Copy` |
| `Result<T, E>` | `Ok(T) \| Err(E)` | Owned tagged ADT; structurally `Copy` when `T: Copy` and `E: Copy` |
| `HashMap<K, V>` | Statically monomorphized generic hash map | Reference type |
| `Box<T>` | Heap-allocated wrapper | Non-`Copy` unique owner of `T` |
| Function | `fn(T1, T2) -> R` | First-class function type |

### 2.3 Special Types

| Type | Description |
|------|-------------|
| `Never` | Type of diverging expressions (`return`, `panic`, `break`). Compatible with every type. |
| `Any` | Not a source-level type. Internal erased values may use `anyref`; `to_string` accepts a compiler-resolved concrete type. |

Parser `NK_ERROR`, unresolved type state, and CoreHIR/MIR recovery sentinels are
compiler-internal states. They are not source-level types and are never used as
`Result` payloads, trait obligations, or public ABI types. `Error` remains an
ordinary nominal user-defined `struct` or `enum` name; it is not reserved for
compiler recovery.

### 2.4 Value vs Reference Semantics

Primitive scalar values are `Copy`. Tuples, arrays, `Option<T>`, and
`Result<T, E>` are structurally `Copy` only when every component is `Copy`.
User-defined structs and enums require an explicit `impl Copy for T {}`;
their field layout does not implicitly opt them in.

Non-`Copy` values move at an ownership boundary and a later use is a type
error. `Box<T>` is always a unique, non-`Copy` owner. `Slice<T>` is a
non-`Copy` read-only view; `as_slice` retains the source binding and stores a
backend-specific reference plus a bounded range. These rules do not depend on
whether the backend stores the value as a Wasm GC object, a tagged/unboxed
value, or a linear-memory record. See [ADR-059](../adr/ADR-059-language-v2-ownership-copy-mutability.md)
and [ADR-060](../adr/ADR-060-language-v2-owned-adts-box-slice.md).

A `let` binding is an ownership boundary: `let b = a` moves a non-`Copy` `a`.
A function call is not: a non-`Copy` argument is borrowed for the duration of
the call and the caller's binding stays usable. Mutation across a call requires
a `mut` parameter and a mutable place at the call site; a value that a callee
stores or returns is shared, not copied
([ADR-088](../adr/ADR-088-language-v2-call-boundary-borrow-and-mut-parameters.md)).
The call-boundary rules are specified in §3.5.1 and are not yet enforced by the
current compiler.

### 2.5 Type Inference

- Integer literals without suffix default to `i32`.
- Float literals without suffix default to `f64`.
- A suffix selects the literal's concrete type; a type annotation does not
  implicitly convert a literal to another numeric type.
- `let` bindings infer their type from the initialiser expression.
- Empty collections require an explicit type argument or annotation
  (e.g., `Vec::new<i32>()` or `let v: Vec<i32> = Vec::new<i32>()`).
- Functions with no declared return type default to `()`.

### 2.6 Numeric Conversion and Literal Inference

Distinct concrete numeric types never undergo implicit promotion or coercion.
After literal inference, arithmetic, comparison, and bitwise operands must
have the same concrete numeric type. The same rule applies to assignments,
function arguments, return values, and generic arguments.

Use a literal suffix when the literal itself should have another type:

```ark
let count: i64 = 1i64
let ratio: f64 = 1.0f64
let total: i64 = 1i64 + 2i64
```

Conversion of a concrete value must be explicit and its mode must be visible:

| Source → target | Canonical API | Semantics |
|---|---|---|
| `i32 → i64` | `value.into()` | lossless widening |
| `i64 → i32` | `value.to_i32_checked()` | `Option<i32>`, `None` outside range |
| `i64 → i32` | `value.to_i32_wrapping()` | two's-complement modulo at i32 width |
| `i64 → i32` | `value.to_i32_saturating()` | clamp to i32 bounds |
| `f32 → f64` | `f32_to_f64(value)` | lossless widening |
| `f64 → f32` | `f64_to_f32(value)` | explicit IEEE/Wasm rounding |

`From` / `Into` are reserved for lossless conversions. Existing prelude
`*_to_*` functions remain explicit compatibility APIs during migration; they
do not enable implicit conversion. Integer aliases such as `u8` and `i16`
currently share compiler representation with `i32`, but their explicit
wrapper semantics remain part of the compatibility surface. See
[ADR-067](../adr/ADR-067-language-v2-numeric-conversion.md).

### 2.7 Generics

Functions, structs, and enums may be parameterised by type variables:

```text
fn identity<T>(x: T) -> T { x }
struct Pair<A, B> { first: A, second: B }
enum Either<L, R> { Left(L), Right(R) }
```

Type parameter bounds are supported by the current trait system:

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
fn print_value<T: Display>(x: T) { … }
```

Generic items have an ordered sequence of type parameters. The language does
not impose a small semantic maximum; backend or toolchain resource limits are
implementation issues and must not change type-checking meaning. The
three-parameter `generic_three_param.ark` fixture is a regression check for
this contract.

Generics are compiled via **monomorphisation** — specialised code is
generated for each concrete type instantiation (ADR-003).

### 2.8 Let-Generalization (TypeScheme) <!-- stability: provisional -->

The type checker supports **let-generalization**: certain `let` bindings
are wrapped in an internal `TypeScheme` that quantifies over free type
variables. Each use of a generalized binding instantiates fresh type
variables, enabling local polymorphic helpers. Without generalization,
every use of the binding shares the same type variable state.

#### Generalization rule

Arukellt uses a conservative **non-expansive/value restriction** until an
effect system can classify evaluation effects. A `let` binding is generalized
only when the binding is immutable and its initializer is a `NonExpansive`
expression. The type checker also removes type variables that are free in the
typing environment or in a declared generic parameter; only the remaining
variables are quantified.

The `NonExpansive` forms are:

- literals: `NK_INT_LIT`, `NK_FLOAT_LIT`, `NK_STRING_LIT`, `NK_BOOL_LIT`,
  `NK_CHAR_LIT`
- closure/lambda literals: `NK_CLOSURE`
- an identifier referring to an already generalized, immutable binding

An identifier is not eligible merely because its AST kind is `NK_IDENT`.
Parameters, mutable bindings, and aliases of monomorphic values remain
monomorphic. `let mut` is never generalized, even when its initializer is a
literal or closure.

The following forms are **expansive** and are stored monomorphically:

- function calls, method calls, and index expressions
- tuple, array, array-repeat, and struct literals (aggregate allocation)
- blocks, `if`, `match`, comprehensions, assignments, unary expressions, and
  binary expressions
- paths and any expression not listed as `NonExpansive`

Known-pure calls are still expansive until an effect system is adopted. A
backend allocation for a literal or closure does not change its source-level
classification; Wasm GC and linear-memory lowering share this rule. Aggregate
allocation, calls, and mutable state therefore cannot gain polymorphism from a
backend-specific optimization.

#### TypeScheme representation

Internally, a `TypeScheme` consists of:
- `vars`: quantified type variable names
- `body`: the body type, which may reference the quantified variables

The `generalize` function collects free (unbound) type variables from the
inferred type, subtracts variables free in the current environment, and builds
a scheme. The `instantiate_scheme` function creates fresh type variables for
each quantified variable on each lookup. This environment-aware subtraction
also prevents a closure that captures a mutable value or an outer generic
parameter from quantifying that outer type.

#### Example

<!-- fixture: selfhost/typescheme_poly_let.ark -->
```ark
// `id` is a non-expansive closure -> generalized
let id = |x| x

// Each call gets fresh type variables
let a: i32 = id(42)     // fresh t0, unified with i32
let b: bool = id(true)  // fresh t1, unified with bool
let s: String = id("hello")  // fresh t2, unified with String
```

#### Scope

- Top-level generic function declarations are handled via explicit
  `FnSig.type_params` and are not affected by this let-generalization
  mechanism.
- Let-generalization is local to function bodies and the scope level where the
  binding appears; type variables owned by that environment are not quantified.
- TypeScheme is an internal compiler representation and does not appear
  in surface syntax.

### 2.9 Traits and Impl Blocks <!-- stability: provisional -->

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
trait Display {
    fn to_string(self: Self) -> String
}

impl Display for Point {
    fn to_string(self: Point) -> String { … }
}
```

- Traits and impl blocks are part of the current parser/type-checker surface;
  there is no edition flag that disables them.
- Static dispatch only (no `dyn`).
- Current protocol traits include `Display`, `Eq`, `Hash`, `Into`/`From`, and
  the provisional `std::error::Error` trait.
- `Error` has the supertrait bound `Display` and the method
  `source(self: Error) -> Option<String>`. It uses static dispatch; `dyn Error`
  and typed dynamic source objects are not part of the current ABI.
- Operator syntax resolves through the canonical traits in
  [ADR-065](../adr/ADR-065-language-v2-operator-trait-resolution.md), not
  through inherent methods or legacy magic names. The mapping is:

  | Syntax | Canonical trait method |
  |--------|------------------------|
  | `a + b`, `a - b`, `a * b`, `a / b`, `a % b` | `Add::add`, `Sub::sub`, `Mul::mul`, `Div::div`, `Rem::rem` |
  | `-a`, `!a`, `~a` | `Neg::neg`, `Not::not`, `BitNot::bitnot` |
  | `a & b`, `a \| b`, `a ^ b`, `a << b`, `a >> b` | `BitAnd::bitand`, `BitOr::bitor`, `BitXor::bitxor`, `Shl::shl`, `Shr::shr` |
  | `a == b`, `a != b` | `Eq::eq` (`!=` negates the result) |
  | `a < b`, `a <= b`, `a > b`, `a >= b` | `Ord::cmp`, projected from `Ordering` |
  | `a[i]`, `a[i] = v`, `*a`, `*a = v` | `Index`, `IndexMut`, `Deref`, `DerefMut` |

  `&&` and `||` remain built-in short-circuit operators. `__add`, `__sub`,
  `__mul`, `__div`, `__rem`, `__eq`, and `__cmp` are not operator candidates.
  The compiler's scalar opcode path is an implementation fast path for the
  same trait protocol. The compiler chooses private backend symbols for methods;
  their spelling is not part of the language or stable ABI contract.

---

## 3. Expressions <!-- stability: stable -->

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

For a user-defined or generic operand, the canonical trait obligation must be
satisfied and the operator's operands and result follow that trait's method
signature. Scalar operators retain the built-in numeric rules below as the
backend fast path for the same protocol. Both operands of arithmetic and
bitwise operators must have the same numeric type after literal inference;
there is no coercion step, and the result type is the same as the operands.

Comparison operators return `bool`. Logical operators require `bool`
operands and return `bool`.

#### 3.3.1 Scalar arithmetic edge semantics

For `i32` and `i64`, `+`, `-`, and `*` wrap at the operand width. Signed `/`
and `%` trap when the divisor is zero; signed `MIN / -1` also traps, while
`MIN % -1` is zero. Shift counts use the low 5 bits for `i32` and the low 6
bits for `i64`; right shift is arithmetic for signed operands. These rules
match the corresponding WebAssembly integer operations.

`f32` and `f64` use IEEE 754 binary32 and binary64 WebAssembly arithmetic.
Floating-point division by zero produces the IEEE result rather than an
integer trap. NaN compares unequal to every value, including itself; `!=` is
true for NaN and all ordered comparisons involving NaN are false. NaN payload
bits are not a source-level contract. `f64_to_f32` uses WebAssembly's explicit
binary32 rounding and `f32_to_f64` is an exact widening conversion; see
[ADR-068](../adr/ADR-068-language-v2-arithmetic-evaluation-semantics.md).

#### 3.3.2 Evaluation order

Evaluation is left-to-right where the syntax contains an ordered sequence.
Call receivers are evaluated before arguments, and arguments are evaluated in
source order. Binary operands, aggregate fields/elements, and an index's base
then index are evaluated in source order. `&&` and `||` retain left-to-right
short-circuit evaluation.

An assignment evaluates its place (including field and index bases), then its
right-hand side, then performs the store. A failed or trapping place
evaluation prevents the right-hand side and store. This order is shared by
the `wasm32-gc` and `wasm32` lowering paths and is not an implementation detail.

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

#### 3.5.1 Argument Passing and Parameter Modes <!-- stability: unimplemented -->

Specified by [ADR-088](../adr/ADR-088-language-v2-call-boundary-borrow-and-mut-parameters.md).

```text
fn read_only(p: Point) -> i32 { p.x }        // shared parameter
fn bump(mut p: Point) { p.x = p.x + 1 }      // mutable parameter

let a = Point { x: 1 }
read_only(a)          // OK: `a` is borrowed for the call, not moved
let mut b = Point { x: 1 }
bump(b)               // OK: `b` is a mutable place
bump(a)               // error: `a` is an immutable binding
```

- **Borrow, not move.** Passing a non-`Copy` value borrows it for the
  duration of the call; the caller's binding is not moved. Passing a binding
  that was already moved is E0212. `Copy` values are copied.
- **Shared parameters.** `p: T` is a shared borrow. The callee must not assign
  to a place reachable through `p`, pass `p` to a `mut` parameter, or call a
  `mut self` method on it.
- **Mutable parameters.** `mut p: T` is a mutable borrow; mutation through `p`
  is visible to the caller. Reassigning `p` itself only rebinds the callee's
  local name. For a `Copy` type, `mut p` mutates the callee's copy only.
- **Mutable places.** An argument for a `mut` parameter must be a `let mut`
  binding, a `mut` parameter or `mut self` of the enclosing function, a field
  or element place rooted at one of these, or a temporary (call result,
  literal, constructor expression).
- **No overlapping borrows.** The same root place must not be passed to a
  `mut` parameter and to another parameter in one call. The check is syntactic
  on the place path.
- **Escape shares.** A callee may return, store, or `move`-capture a borrowed
  parameter; the value is shared as a reference, not copied. The language
  guarantees that no mutation happens through a path rooted at an immutable
  binding or a shared parameter; a mutation through another alias is visible
  through every alias, identically on every target. `Box<T>` is excluded: a
  borrowed `Box<T>` parameter cannot be returned, stored, or `unbox`ed.
- **Function types.** The mode is part of the signature and of function types
  (`fn(mut Vec<i32>) -> i32`). A function value with a shared parameter is
  assignable where the same position is `mut`; the reverse is rejected.
  Closure parameters use the same form (`|mut v| …`).

**Current implementation.** The compiler does not parse `mut` parameters yet,
accepts mutation through any parameter or receiver, and does not report E0212
for every use of a moved binding as an argument. Tracked by
[#951](../../issues/open/951-mut-parameter-mode-and-call-borrow-checks.md) and
[#944](../../issues/open/944-move-after-use-not-diagnosed.md); see
[../current-state.md](../current-state.md).

### 3.6 Method Call

```
expr.method(arg1, …)
```

Resolved via `impl` blocks. The receiver is the first parameter.
A `self: T` receiver is a shared borrow and a `mut self: T` receiver is a
mutable borrow; calling a `mut self` method requires a mutable place as the
receiver (§3.5.1, ADR-088).
The compiler chooses private backend symbols for methods. A concrete symbol
spelling is not part of the language or stable ABI contract.

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

`expr?` early-returns from the enclosing function when the operand is a
failure variant. Two forms are defined:

**`Result<T, E_source>`** — the enclosing function must return
`Result<_, E_target>`. On `Ok(v)`, the value of `expr?` is `v`. On `Err(e)`:

- if `E_source` and `E_target` are the same type, return `Err(e)` (identity)
- otherwise return `Err(From::from(e))`, resolved via the canonical `From`
  trait for language-syntax desugaring (import not required; ADR-039 D2)
- if no unique `From<E_source> for E_target` implementation exists, type
  checking stops with E0210; `TryFrom` and `TryInto` are not fallback candidates

**`Option<T>`** — the enclosing function must return `Option<_>`. On
`Some(v)`, the value of `expr?` is `v`. On `None`, return `None`. There is
no error-type conversion. Converting `Option` to `Result` is out of scope
for `?` (use an explicit `ok_or` / `ok_or_else`).

Any other operand type, or a mismatch between the operand and the enclosing
return type (`Result` vs `Option`), is a type error with E0210. This validation
happens before CoreHIR/MIR lowering and does not depend on declaration order or
the selected Wasm representation.

Evidence: `tests/fixtures/question_mark/basic_propagate.ark`,
`from_error.ark`, `option_propagate.ark`,
`diagnostics/question_from_missing.ark`, and
`question_mark/user_error_type.ark`.

Fallible conversion is explicit and separate from `?`. ADR-084 fixes the
experimental v2 surface as the `TryFrom<T>` associated function (called as
`i32::try_from("42")`, with `String` as the initial error type); it is
available for `String -> i32/i64/f64` and range-checked `i64 -> i32` narrowing
(#708). Associated overloads are selected by the source argument type, so
same-target impls dispatch independently. The `TryInto::try_into` method form
is not part of the initial v2 surface; selecting its impl requires
target-type-directed trait method resolution, which is not adopted yet.

### 3.10 If Expression

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
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

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
if a { … } else if b { … } else { … }
```

### 3.11 Match Expression

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
match scrutinee {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```

All arms must produce the same type. Guards are optional boolean
expressions.

A `match` expression must be provably exhaustive (ADR-082). The
checker accepts a match only when one of the following holds:

- an unguarded wildcard (`_ =>`) or binding-identifier arm exists;
- the scrutinee is a finite domain (bool, Option, Result, or a known
  enum) and every variant is covered by unguarded arms whose payload
  sub-patterns are all irrefutable (`_` or a binding identifier);
- an unguarded tuple or struct pattern whose sub-patterns are all
  irrefutable covers the whole scrutinee.

Guarded arms never contribute to coverage, and a variant pattern covers
its variant only when all payload sub-patterns are irrefutable — `Some(v)`
covers `Some`, but `Some(0)` does not. A match that cannot be proven
exhaustive is a compile-time diagnostic (non-exhaustive match), so a
program where no arm matches is rejected before code generation on both
backends.

An arm pattern may be an or-pattern (p1 | p2 => …). Each alternative
contributes coverage independently and is compared in turn against the
scrutinee (tag for variant alternatives, value equality otherwise).
Alternatives are discriminators only: an alternative that would bind a
variable — a bare identifier that is not a unit-variant name, or a
variant alternative with a binding payload such as Some(v) — is a
compile-time diagnostic, as are nested composite alternatives
(#901). Payload sub-patterns that are not bindings, such as
Shape::Circle(_), are permitted.

A variant pattern's payload sub-patterns participate in arm selection,
not only in coverage accounting (#902). After the tag compares equal,
each non-binding child — a literal, a unit-variant name, a tuple, or a
nested variant — is compared against the extracted payload field, so
Some(5) does not match Some(7). Composite children still introduce
bindings for their inner identifiers (Some((k, _)) binds k; Some(Some(v))
binds v). A struct sub-pattern inside a variant payload is not yet
implemented and is a compile-time diagnostic rather than a tag-only
match.

### 3.12 Block Expression

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
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

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
Point { x: 1, y: 2 }
Point { x: 1, ..base }    // struct update syntax
```

### 3.14 Tuple Expression

<!-- fixture: variables/tuple.ark -->
```ark
(1, "hello", true)
```

### 3.15 Array Expressions

<!-- fixture: arrays/array_literal.ark --><!-- fixture: arrays/array_repeat.ark -->
```ark
[1, 2, 3]        // array literal
[0; 10]           // array repeat: 10 elements, each 0
```

### 3.16 Closure Expression

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
|x| x + 1
|x: i32, y: i32| -> i32 { x + y }
move |prefix: String| -> String { prefix }
```

Closures capture variables from the enclosing scope. Parameter types
may be annotated or inferred.

`move` is contextual in this position: `move |...|` selects move capture,
while an identifier named `move` outside a closure prefix keeps ordinary name
resolution. A closure without `move` captures a non-`Copy` outer binding by
shared borrow, or by mutable borrow when its body mutates that binding.
`Copy` bindings are copied independently of the capture mode.

Shared-borrow captures are valid only within their enclosing function.
Returning such a closure, storing it in an aggregate, or passing it as a
callback is rejected with E0213. A mutable-borrow capture (a body that
rebinds its source) may escape: the source place is promoted into a shared
heap cell at the capture point, so the managed environment carries a live
alias and writes through it are visible on both sides (ADR-081).
Reassigning or moving the borrowed source is rejected with E0214.
The rules are source-level and do not depend on Wasm GC or linear-memory
representation; the promoted cell lowers to a one-field GC struct on
`wasm32-gc` and to an equal-width linear-memory slot on `wasm32`.

A nested closure captures names from its whole lexical scope, including
bindings an intermediate closure does not itself reference. Each such
intermediate frame forwards the name as a transport slot in its
environment; the frame does not acquire a borrow or move on names it
only forwards (ADR-083). When a descendant rebinds the source, the
source place is promoted into a shared cell and every forwarded slot
carries the cell reference, so writes through a nested borrow reach the
defining binding on both backends.

### 3.17 Loop Expression

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
loop {
    if done { break value }
}
```

`loop` produces a value via `break value`.

### 3.18 Return and Control Flow

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
return expr     // early return from function
return          // return ()
break           // exit loop
break expr      // exit loop with value
continue        // skip to next loop iteration
```

### 3.19 Assignment

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
x = expr
obj.field = expr
v[i] = expr
```

Assignment target must be a `let mut` binding, or a field or index place
rooted at a `let mut` binding or a `mut` parameter; otherwise it is E0207
(ADR-059). Places rooted at a shared parameter or a `self: T` receiver are
not mutable (ADR-088, §3.5.1; not yet enforced). Assignment returns `()`.

### 3.20 Array Comprehension <!-- stability: provisional -->

A lightweight collection-construction form that desugars into an explicit `for` loop
with a `Vec` builder:

<!-- fixture: spec_examples/array_comprehension.ark -->
```ark
[x * 2 for x in xs]
[x * 2 for x in xs if x > 0]
```

The canonical syntax is `[<expr> for <ident> in <iter>]`, with an optional filter
`if <expr>` after the iterable.

<!-- fixture: spec_examples/array_comprehension.ark -->
```ark
let doubled = [x * 2 for x in xs]             // map only
let evens = [x * 2 for x in xs if x % 2 == 0]  // map + filter
```

**Desugaring** (no filter):

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
{
    let result: Vec<ElemType> = Vec::new<ElemType>()
    let iterator = ITER.into_iter()
    loop {
        match iterator.next() {
            Some(VAR) => result.push(ELEM),
            None => break,
        }
    }
    result
}
```

With filter:

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
{
    let result: Vec<ElemType> = Vec::new<ElemType>()
    let iterator = ITER.into_iter()
    loop {
        match iterator.next() {
            Some(VAR) => {
                if FILTER {
                    result.push(ELEM)
                }
            }
            None => break,
        }
    }
    result
}
```

**Type rules:**
1. `ITER` must provide the canonical `IntoIterator` conversion for some element type `E`.
   A source without that implementation is a type error.
2. The loop variable `VAR` is bound to type `E`.
3. `ELEM` is type-checked in the loop scope. Its type `T` becomes the element type of the result.
4. If `FILTER` is present, it must evaluate to `bool`.
5. The overall comprehension has type `Vec<T>`.

**Restrictions:** One generator only (no nested `for`). `break` and `continue` inside
a comprehension are not allowed (they would refer to the implicit loop, which is
confusing).

The generated `into_iter`, `next`, and `Vec::push` calls are compiler-owned receiver
references. Ordinary source bindings with those names cannot capture a comprehension.
The filter and element expression keep the same evaluation order as the explicit core
form above.

---

## 4. Statements <!-- stability: stable -->

### 4.1 Let Binding

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
let name = expr
let name: Type = expr
let mut name = expr
let mut name: Type = expr
```

Destructuring via pattern:

<!-- fixture: variables/tuple.ark -->
```ark
let (a, b) = (1, 2)
```

The `mut` qualifier allows the binding to be reassigned.

### 4.2 Expression Statement

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
expr;
expr      // semicolon optional
```

### 4.3 While Loop

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
while cond {
    body
}
```

### 4.4 Loop

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
loop {
    body
}
```

### 4.5 For Loop

Three forms of iteration:

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
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

For a concrete source with an `IntoIterator` implementation, the v2
iteration contract is equivalent to converting the source first and then
pulling the resulting iterator:

```ark
let iterator = expr.into_iter()
loop {
    match iterator.next() {
        Some(item) => body,
        None => break,
    }
}
```

The current implementation applies this trait boundary to `Vec<i32>`,
`Range`, `RangeInclusive`, and their concrete iterator types. The legacy
`values(v)` form remains an explicit compatibility path, and concrete types
without an `IntoIterator` implementation retain the structural `next()`
fallback until their trait implementation is available.

The iterable source may be a direct call or method expression, not only a
pre-bound local. For example, `for item in Range::new(0, 3)` and
`for item in RangeInclusive::new(0, 2).into_iter()` use the same
conversion and `next()` sequence as their equivalent local-binding forms.
Concrete source dispatch remains static; this does not imply dynamic dispatch
for an unregistered source. A generic function bounded by `Iterator` may use
its parameter directly as a `for` source. After monomorphization, the
`iterator_vec_iter_for_loop.ark` fixture verifies `T = VecIter<i32>` through the
same identity `IntoIterator::into_iter` and canonical `next` sequence.
The corresponding generic `T: IntoIterator` case is also monomorphized for
concrete `Vec<i32>` sources; the same fixture verifies `Vec::into_iter` before
`VecIter<i32>::next`.
`iterator_range_basic.ark` extends this evidence to direct `Range` and
`RangeInclusive` expressions, preserving half-open and closed-range item
sequences through the same generic `for` contract.

The v2 producer contract is item-preserving: `Iterator` requires the
associated `Item`, `next`, `collect`, and `count` methods. Numeric consumers
are separate capabilities rather than requirements on every producer.
`IteratorSum<Item, Output>` provides `sum`, and
`IteratorFold<Item, Accumulator>` provides `fold`; the current numeric lane
implements `IteratorSum<i32, i32>` and `IteratorFold<i32, i32>` for the concrete
numeric adapters. A consumer call is legal only when the receiver satisfies
the corresponding capability trait. A non-numeric iterator is not required to
invent a numeric result or to pack its item into an integer: for example,
`VecIter<String>.sum()` is rejected during type checking with E0200 rather than
lowering to a runtime stub or trap. The negative contract is pinned by
`selfhost/iterator_string_numeric_consumer.ark`.

`Vec<String>` implements `IntoIterator` through the item-preserving
`VecIter<String>` adapter. `iterator_string.ark` verifies direct and generic
`Iterator`/`IntoIterator` calls for `next`, `collect`, and `count`, concrete
`Vec<String>::iter()` and `Vec<String>::from_iter`, method-shaped `collect`,
and direct `for` iteration. `iter()` and `into_iter()` select the same
`VecIter<String>` producer boundary; the former is the canonical collection method
entry point and does not reinterpret String references as numeric payloads.
`VecIter<String>::take(n)` keeps the same item-preserving contract through the
source-wrapping `TakeIter<VecIter<String>>` adapter; the fixture verifies its
lazy bounded `collect` path without introducing an integer item
representation.
`VecIter<String>::skip(n).take(m)` composes the same state transitions through
`TakeIter<SkipIter<VecIter<String>>>`: skip is applied before the bounded pull, with the
`String` item type preserved throughout.
The GC component and linear-memory Wasm artifacts validate and run with the
same assertions. The `for` lowering chooses the typed `Option::Some` payload
layout for the iterator item, so a String reference is never read through the
numeric i32 payload layout.

The String producer also supports lazy callback adapters through the
item-generic adapter types shared with the i32 lane. `VecIter<String>::map`
accepts `fn(String) -> String` and returns `MapIter<String, String>`;
`VecIter<String>::filter` accepts `fn(String) -> bool` and returns
`FilterIter<String>`. Both retain `Iterator::Item = String` and delegate
collection to the canonical `Iterator::collect` path. The positive contract is
pinned by `stdlib_trait/iterator_string_callbacks.ark`. Because a callback owns
its String argument, `filter` passes a cloned handle to the predicate and
yields the original accepted item. On `wasm32-gc`, these fields and calls use
typed `call_ref` signatures; on linear-memory `wasm32`, the same source calls
use the callback table and `call_indirect`. The representation differs, but
the source-level item order, acceptance, and collection result are identical.
The `FromIterator` impls for these adapter instantiations are also available
through the source-level `Vec::from_iter` associated function. The compiler
selects the source-qualified `Vec_String::from_iter__MapIter_String_String`
or `Vec_String::from_iter__FilterIter_String` implementation after
monomorphizing the adapter source; `iterator_string_callbacks.ark` verifies
both calls on GC and linear-memory Wasm. The same fixture verifies a
`T: Iterator` helper whose String adapter instantiation resolves to the same
static implementation. This is bounded static monomorphization evidence, not a
claim that arbitrary item representations or dynamic collection dispatch are
complete.
The String lane also composes `map` followed by `filter` without an
intermediate vector through `MapFilterIter<String, String>`. The adapter
preserves the mapped item, passes a clone to the owning predicate, and exposes
the same `collect`, `IntoIterator`, and `FromIterator` contracts; GC and
linear-memory fixtures verify both direct collection forms.
The reverse `filter` followed by `map` composition runs through
`FilterMapIter<String>`: the predicate receives a clone of the source item,
and only an accepted original is passed to the mapping callback. Its `collect`
and `FromIterator` paths use the same target-specific callback
representations. Method-level generics (ADR-079) are inferred from the
callback argument: `VecIter<T>::map<U>(f: fn(T) -> U)` returns
`MapIter<T, U>`, and `FilterIter<T>::map<U>(f: fn(T) -> U)` returns
`FilterMapIter<T, U>`. A concrete-receiver call that leaves `U` unresolved
is a type error. Explicit turbofish-style method type arguments are not part
of the syntax.
The same fixture sends this composed String adapter through a generic
`total_string_lengths<T: IntoIterator>` consumer. Monomorphization emits
`total_string_lengths__FilterMapIter_String`, which performs the adapter's
identity `IntoIterator` conversion before the canonical `next` loop. GC and
linear-memory artifacts validate and run the same length assertion; this is
still bounded evidence for a String adapter instantiation, not arbitrary
dynamic dispatch.
The same callback lane supports bounded compositions through
`TakeIter<MapIter<String, String>>` and `TakeIter<FilterIter<String>>`.
`map(...).take(n)` counts mapped outputs, while `filter(...).take(n)` counts
accepted items and clones the predicate argument before yielding the original
String. Their `collect`, source-qualified `Vec::from_iter`, and generic
`T: IntoIterator` `for` paths are verified on both Wasm targets.
The two callback orders can also be bounded after both stages through
`TakeIter<MapFilterIter<String, String>>` and
`TakeIter<FilterMapIter<String>>`. The bound applies to accepted outputs, and
each owning predicate receives a cloned handle before the value proceeds to
the next callback. Their `collect`, `Vec::from_iter`, and generic
`T: IntoIterator` `for` paths are verified on both targets.
The item-preserving String lane also supports `skip(n).map(f)` and
`skip(n).filter(g)` through `SkipMapIter<VecIter<String>>` and
`SkipFilterIter<VecIter<String>>`, the same item-generic adapters the i32 lane
uses at `VecIter<i32>`.
Both adapters consume the skipped source items before dispatching a callback;
the filter predicate receives a cloned handle and an accepted original item is
yielded. Their `collect`, `Vec::from_iter`, and generic `T: IntoIterator`
consumers are verified on GC and linear-memory Wasm with the same item order
and bounded results. This remains concrete String evidence, not arbitrary
item-generic adapter support.

For a statically known source, type checking requires an `IntoIterator` or
`Iterator` implementation, or a concrete `Type::next` method for the current
structural fallback. A known source that satisfies none of these contracts is
rejected with E0200 (`non-iterable source`); the regression fixture
`selfhost/for_deny_non_iterable.ark` covers the scalar `i32` case. Unresolved
generic sources remain deferred until their concrete obligation can be checked;
the verified generic case is bounded by `Iterator` and then monomorphized.

For the current `Vec<i32>` item lane, the canonical lazy adapter expression is
`v.iter().map(f).filter(g).take(n).collect()`. It evaluates the source through
the iterator protocol and does not require an intermediate collection between
the adapter stages. Other item representations and additional adapter
compositions remain implementation work under the iterator migration issues.
The same concrete method-shaped adapter can be used directly as a `for` source;
`iterator_adapters.ark` verifies `v.iter().map(f).filter(g).take(2)` reaches
the composed adapter's `IntoIterator::into_iter` identity conversion and then
its `next()` path on both published Wasm targets. This does not extend the
contract to arbitrary item representations or dynamic dispatch.
The same fixture instantiates a generic `T: IntoIterator` `for` consumer with
the composed adapter, proving that the identity conversion remains visible
after adapter monomorphization.
The `iter().map(f).filter(g).take(n).collect()` spelling is the only reachable
form: the eager `std::core::iter` constructor and collector frees were retired
(#917), so no helper maintains a second eager traversal.
The concrete `iter().map(f).take(n).collect()` composition is also implemented
by the lazy `TakeIter<MapIter>` adapter; `iterator_adapters.ark` verifies the
two-element `Vec<i32>` case on both published Wasm targets. Arbitrary adapter
combinations and other item representations remain outside this contract.
The concrete `iter().filter(g).take(n).collect()` composition is implemented
by `TakeIter<FilterIter>`: `FilterIter` retains the predicate and `TakeIter`
bounds the remaining count. The fixture also sends this adapter through a generic
`T: IntoIterator` consumer; this evidence remains limited to `Vec<i32>`.
The concrete `iter().skip(n).take(m).collect()` composition is implemented by
`TakeIter<SkipIter>`, which applies the skip state before the bounded pull. Its
identity `IntoIterator` conversion and generic `for` consumer are covered by
the same fixture on both published Wasm targets.
The numeric `Vec<i32>` lane also composes `skip(n).map(f)` and
`skip(n).filter(g)` through `SkipMapIter` and `SkipFilterIter`. Both consume
the skipped source prefix before callback dispatch and retain the canonical
`collect`, `Vec::from_iter`, `IntoIterator`, and numeric consumer paths. The
fixture verifies direct collection, source-qualified construction, generic
`T: IntoIterator` traversal, and matching `sum` results on GC and linear-memory
Wasm. This is concrete numeric evidence, not arbitrary item-generic adapter
support.
The compatibility helpers in `std::seq` (`map_i32_i32`, `filter_i32`,
`take_i32`, `skip_i32`, and `fold_i32_i32`) keep their eager signatures while
delegating to the canonical lazy iterator methods, including
`take` / `skip` and their `TakeIter` / `SkipIter` collectors (#691). The
`seq_map_filter.ark`, `seq_take_skip_fold.ark`, and
`seq_take_skip_collect.ark` fixtures compile, validate, and produce identical
output on `wasm32-gc` and linear-memory `wasm32`.

### 4.6 Semicolons

Semicolons are explicit statement or item terminators. Within a block, the
absence of a semicolon on the last expression makes it the block's tail
expression (return value). A newline may separate statements when the
preceding expression is complete.

At the outermost expression level, a newline or semicolon after a complete
expression terminates that expression. A newline remains a continuation when
the expression is inside an open delimiter or the preceding token requires a
continuation, such as a binary, assignment, or range operator. A semicolon is
never silently discarded while deciding whether two top-level declarations
belong to one syntactic group.

---

## 5. Pattern Matching <!-- stability: stable -->

Patterns appear in `match` arms, `let` bindings, `for` targets, and multi-clause `fn` clause heads.

### 5.1 Wildcard

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
_ => …
```

Matches anything, discards the value.

### 5.2 Variable Binding

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
x => …
```

Binds the matched value to `x`.

### 5.3 Literal Patterns

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
42 => …
"hello" => …
true => …
'a' => …
```

### 5.4 Enum Variant Pattern

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
Some(x) => …
Ok(value) => …
Shape::Circle(r) => …
```

### 5.5 Struct Pattern

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
Point { x, y } => …
Point { x: px, y: py } => …
```

### 5.6 Tuple Pattern

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
(a, b, c) => …
```

### 5.7 Or Pattern

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
1 | 2 | 3 => …
Some(0) | None => …
```

### 5.8 Match Guards

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
x if x > 0 => …
```

---

## 6. Items <!-- stability: stable -->

Items are top-level declarations within a module.

### 6.1 Function Definition

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
fn name(param1: T1, param2: T2) -> RetType {
    body
}

pub fn name<T>(param: T) -> T {
    body
}

export fn component_name(param: i32) -> i32 {
    body
}

pub export fn public_component_name(param: i32) -> i32 {
    body
}
```

- `pub` makes the function visible to other modules.
- `export` makes the function a Component/WIT export root without changing
  source-module visibility. `pub export` applies both contracts.
- `pub` alone never creates a Component or raw Wasm export.
- `export` is valid only on function declarations. Modifier order is
  `[pub] [async] export fn`.
- Return type defaults to `()` if omitted.
- A parameter may be declared `mut name: T` to receive a mutable borrow
  (§3.5.1, ADR-088; not yet parsed by the current compiler).

#### 6.1.1 Multi-Clause Function Definitions <!-- stability: provisional -->

A function may be defined as a group of pattern-matching clauses rather than a single
block body. Each clause consists of `fn <name>(<pattern-params>) -> Ret = <expr>`,
where parameters are **patterns** (not typed identifiers). The body is a single expression
after `=`.

Clauses share the same name, arity, and return type annotation. A clause group
continues only across one or more newline tokens to another clause-form `fn`
with the same name. An explicit semicolon terminates the group; a same-name
clause after that semicolon is a separate declaration and is subject to the
ordinary duplicate-definition rule.
The group desugars to a single `fn` with a `match` body.

<!-- fixture: spec_examples/multi_clause_functions.ark --><!-- fixture: spec_examples/multi_clause_pub.ark -->
```ark
// Integer literal clauses with wildcard catch-all
fn classify(0) -> String = "zero"
fn classify(1) -> String = "one"
fn classify(_) -> String = "other"

// Boolean literal clauses
fn negate(true) -> bool = false
fn negate(false) -> bool = true

// Enum variant clauses (single-param only)
fn unwrap_or_zero(Option::Some(v)) -> i32 = v
fn unwrap_or_zero(None) -> i32 = 0

// String literal clauses
fn greet("hello") -> String = "hi"
fn greet("bye") -> String = "goodbye"
fn greet(_) -> String = "huh?"

// Identifier-binding clause (catch-all, binds the argument)
fn answer(42) -> String = "life"
fn answer(n) -> String = "unknown"

// Single clause with typed identifier (degenerate clause group)
fn identity(x: T) -> T = x

// Pub visibility propagates from first clause
pub fn is_zero(0) -> bool = true
pub fn is_zero(_) -> bool = false
```

**Grouping rules:**
1. **Same name**: every clause must have the same `IDENT`.
2. **Same arity**: every clause must have the same number of parameters.
3. **Compatible return**: all clauses must agree on the return type annotation.
4. **Newline-separated**: clauses must be separated by one or more newlines.
   A semicolon is an item terminator and ends the group.

**Desugaring:** Multi-clause groups are desugared at parse time into a single `fn`
with a synthetic block body containing a `match` expression. All downstream passes
(resolver, typechecker, code generation) see only the desugared form.

Single-parameter multi-clause functions are supported. Multi-parameter clause groups
are not yet supported in this release.

The canonical block-body `fn` syntax (SS6.1) remains supported and interoperates with
clause-style definitions in the same module. Clause syntax is preferred when the
function body is naturally expressed as pattern dispatch on a single scrutinee.

#### 6.1.2 Clause Guards <!-- stability: provisional -->

Multi-clause function heads may carry a **guard expression** after the return type
annotation using `| <expr>` syntax. Guards are boolean expressions evaluated after
pattern matching: the pattern must match first, then the guard is evaluated. If the
guard is `true`, the clause body is chosen; otherwise, evaluation falls through to
the next clause.

<!-- fixture: spec_examples/clause_guards.ark -->
```ark
fn classify(n) -> String | n > 0 = "pos"
fn classify(n) -> String | n < 0 = "neg"
fn classify(_) -> String = "zero"
```

Guard expressions reuse the same evaluation model as `match` arm guards (SS5.8) and
may reference the function parameter bindings established by the clause pattern.
The guard must evaluate to `bool`.

<!-- fixture: spec_examples/clause_guards.ark -->
```ark
struct Point { x: i32, y: i32 }

fn describe(pt) -> String | pt.x > 0 = "right"
fn describe(pt) -> String | pt.x < 0 = "left"
fn describe(_) -> String = "center"
```

**Evaluation order:** match pattern -- bind parameters -- evaluate guard -- choose body.

#### 6.1.3 Where Clauses <!-- stability: provisional -->

A multi-clause function group may end with a `where` block declaring shared value
bindings visible across all clauses. Each binding is `name = expr`; bindings are
separated by one or more newlines or by a semicolon.

<!-- fixture: spec_examples/where_clause_magnitude.ark -->
```ark
fn magnitude_label(p: Point) -> String | m == 0 = "origin"
fn magnitude_label(p: Point) -> String | m < 10 = "near"
fn magnitude_label(_) -> String = "far"
where
    m = p.x * p.x + p.y * p.y
```

Where-bindings are evaluated in source order before the clause dispatch. A binding
may reference the synthetic function parameter and earlier `where` bindings, but
not a pattern binding belonging only to a later or selected clause. Each clause's
guard and body may reference its own pattern bindings and all `where` bindings.
The bindings are not visible outside the function. A semicolon before `where`
terminates the clause group, so it does not attach to that group.

**Initial restriction:** non-recursive value bindings only. Local helper functions
and mutual recursion are deferred.

### 6.2 Struct Definition

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
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

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
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

### 6.4 Trait Definition <!-- stability: provisional -->

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
trait Name {
    fn method(param: T) -> R
}

pub trait Name<T> {
    fn method(self, x: T) -> T
}
```

### 6.5 Impl Block <!-- stability: provisional -->

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
impl Name {
    fn method(self) -> R { … }
}

impl Trait for Name {
    fn method(self) -> R { … }
}

impl<T: Bound> Trait for Ctor<T> {
    fn method(self) -> R { … }
}
```

Impl type parameters are introduced only by the `impl<…>` binder, for trait
and inherent impls alike. Every other name in the impl head resolves as an
ordinary type name, so `impl Copy for S {}` targets a declared `struct S`.
Name length or case never decides whether a name is a type or a type
parameter ([ADR-089](../adr/ADR-089-language-v2-explicit-impl-type-parameter-binders.md)).
Parametric and blanket impls are classified as in
[ADR-078](../adr/ADR-078-language-v2-parametric-trait-impl.md).

**Current implementation.** A binder-less head still treats single-uppercase
names as implicit type parameters, which makes `impl Trait for S {}` with a
`struct S` fail as a blanket impl (E0216). Removal is tracked by
[#946](../../issues/open/946-single-uppercase-implicit-impl-param.md).

---

## 7. Module System <!-- stability: provisional -->

### 7.1 Import Syntax

`use` is the canonical source-module declaration. `import` is retained only
as a migration form for local source modules; it is not a second source-module
namespace.

**`use`** — source-module import syntax:

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
use std::host::stdio
use std::core as c
use std::collections::string::{split, join}
```

The final path segment is the default module alias. `as` replaces that alias
for the current module. A brace group selects public items from the containing
module; the selected items are available through their short names. Item-level
aliases inside a brace group are not part of the current grammar.

`pub use` re-exports the selected public items. A private source item remains
private and cannot become visible through a re-exporting facade.

**`import`** — compatibility syntax for local source modules:

<!-- skip-doc-check reason="syntax/notation sketch with metavariables; not a runnable program" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
import math
import utils as u
```

`import` loads a local source module and accepts the same optional module alias
as `use`. It emits `W0101` at parse phase. Qualified access via
`math::add(…)` remains accepted so existing source can migrate incrementally.

String-literal `import` is the Layer C Component Model / WIT boundary. Its
identifier uses `namespace:package/interface[@version]`; a supplied full
version must match the loaded WIT document exactly, while an omitted version
resolves to that document's version. `use` and `::` remain the Layer S source
namespace and never become WIT identifiers. The source/export split and
canonical identity are defined by [ADR-071](../adr/ADR-071-component-export-visibility-boundary.md).
Normative layer contract: [../spec/import-system.md](../spec/import-system.md).

### 7.2 Module Structure

Each `.ark` file is a module. A module consists of:

1. `//!` inner doc comments
2. Import / use declarations
3. Item definitions (`fn`, `struct`, `enum`, `trait`, `impl`)

### 7.3 Visibility

- Items are **private** by default (visible only within the defining module).
- The `pub` keyword makes an item visible to importing modules.

### 7.4 Prelude

The v2 stable Prelude is the fixed set in the `[prelude_policy]` table of
`std/manifest.toml`:

**Types**: `Option`, `Result`, `String`, `Vec`

**Values**: `Some`, `None`, `Ok`, `Err`

**Lexical literals**: `true`, `false` (not injected values)

**Stable functions**: `panic`, `assert`, `String_from`, `String_new`,
`char_to_string`

Only this stable set is auto-imported (`legacy_auto_import = false`, #878).
Every other `prelude = true` declaration requires an explicit
`use std::prelude::{name}`; the resolver reports a missing import as
`undefined name …; no longer auto-imported`. Retirement of the remaining
compatibility free functions is tracked by #703/#917 (ADR-046).

---

## 8. Operator Precedence <!-- stability: stable -->

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

## 9. Standard Library API <!-- stability: see individual entries -->

Authoritative source: `std/manifest.toml` (263 entries).

Functions marked **prelude** are available in every module without an
import. Non-prelude functions require a `use` or `import` declaration.

### 9.1 String — Construction & Operations (prelude)

| Signature | Description |
|-----------|-------------|
| `String_from(s: String) -> String` | Construct from literal |
| `String_new() -> String` | Empty string |
| `byte_at(s: String, i: i32) -> i32` | Raw UTF-8 byte at a byte index |
| `byte_len(s: String) -> i32` | UTF-8 byte length |
| `eq(a: String, b: String) -> bool` | Equality |
| `clone(s: String) -> String` | Clone |
| `starts_with(s: String, prefix: String) -> bool` | Prefix test |
| `ends_with(s: String, suffix: String) -> bool` | Suffix test |
| `to_lower(s: String) -> String` | Lowercase |
| `to_upper(s: String) -> String` | Uppercase |
| `slice(s: String, start: i32, end: i32) -> String` | Substring by index |
| `trim(s: String) -> String` | Strip leading/trailing whitespace |
| `contains(s: String, sub: String) -> bool` | Substring test |
| `char_at(s: String, i: i32) -> i32` | Deprecated raw byte compatibility API; index is a byte index |
| `substring(s: String, start: i32, end: i32) -> String` | Substring |
| `replace(s: String, from: String, to: String) -> String` | Replace all |
| `split(s: String, delim: String) -> Vec<String>` | Split by delimiter |
| `join(v: Vec<String>, sep: String) -> String` | Join with separator |
| `push_char(s: String, c: char)` | Append character (mutates) |
| `is_empty(s: String) -> bool` | Empty test |

The deprecated char_at entry returns a raw byte for compatibility. Use byte_at for
byte inspection and codepoint_at for a Unicode scalar value. byte_at and byte_len
remain prelude compatibility names for the byte API.

### 9.1.1 std::text — Unicode and byte operations

The std::text module exposes the explicit string domains. chars returns one valid
UTF-8 String per Unicode scalar value, including supplementary scalars and combining
marks as separate elements. slice_chars uses scalar indices; slice_bytes uses byte
indices and rejects non-boundary ranges. len_chars counts scalars, not grapheme clusters.

### 9.2 Type Conversions (prelude)

| Signature | Description |
|-----------|-------------|
| `i32_to_string(n: i32) -> String` | i32 → String |
| `i64_to_string(n: i64) -> String` | i64 → String |
| `f64_to_string(n: f64) -> String` | f64 → String |
| `bool_to_string(b: bool) -> String` | bool → String |
| `char_to_string(c: char) -> String` | char → String |
| `to_string<T>(x: T) -> String` | Canonical compiler-resolved conversion; `T` is not a user-visible `Any` type |
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

<a id="94-control"></a>

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

### 9.7 Vec — Sorting

The monomorphic prelude sort frees (`sort_i32`, `sort_i64`, `sort_f64`,
`sort_String`) were removed in the ADR-087 Wave 2 (issue #703 / #917). Use the
`Vec` method `v.sort_by(cmp)` from `std::collections::vec` for the verified
`Vec<i32>` / `Vec<String>` lanes. The `i64` / `f64` `Vec` receiver lanes still
emit invalid Wasm; `std::seq::sort_by(v)` is the reachable generic
`Ord`-based fallback until those lanes land.

### 9.8 Vec — Higher-Order Functions

The monomorphic higher-order prelude frees (`map_*`, `filter_*`, `fold_*`,
`any_*`, `find_*`) were removed in the ADR-087 Wave 2. Use the lazy iterator
methods from `std::core::iter`:

| Signature | Replacement |
|-----------|-------------|
| `map_i32_i32`, `map_String_String` | `v.iter().map(f).collect()` |
| `filter_String` | `v.iter().filter(f).collect()` |
| `fold_i32_i32` | `v.iter().fold(init, f)` |
| `any_i32`, `any_String` | `v.iter().any(f)` |
| `find_i32`, `find_String` | `v.iter().find(f)` |

The `i64` / `f64` numeric consumers are not lowered by the iterator backend
yet; use an explicit `while` loop until those lanes land. The provisional
`filter_i32` remains callable (individual deferral).

### 9.9 Vec — Aggregation & Mutation

The aggregation and mutation frees (`sum_i32`, `product_i32`, `contains_*`,
`reverse_*`, `remove_i32`) were removed in the ADR-087 Wave 2. Use
`v.iter().sum()`, `v.iter().fold(1, mul)`, `v.iter().contains(x)`,
`v = v.iter().rev().collect()`, and `v.drain(i, i + 1)` respectively. The
`Vec<String>` reverse/contains lanes use an explicit swap / loop until the
lane lands; `s.contains(sub)` replaces the String substring helper.

<a id="910-option"></a>

### 9.10 Option (prelude)

| Signature | Description |
|-----------|-------------|
| `unwrap(o: Option<T>) -> T` | Extract or panic |
| `unwrap_or(o: Option<T>, default: T) -> T` | Extract or default |
| `unwrap_or_else(o: Option<T>, f: fn() -> T) -> T` | Extract or compute |
| `is_some(o: Option<T>) -> bool` | Some test |
| `is_none(o: Option<T>) -> bool` | None test |
| `expect(o: Option<T>, msg: String) -> T` | Extract or panic with message |
| `map_option_i32_i32` (removed in Wave 2) | `o.map(f)` via `std::core::container` |
| `map_option_String_String` (retired) | `match o { Some(v) => Some(f(v)), None => None }` |

<a id="911-result"></a>

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
| `Box_new(x: T) -> Box<T>` | Consume a non-`Copy` `x` and wrap it in a unique owner |
| `unbox(b: Box<T>) -> T` | Consume the unique owner and return its `T` |

### 9.12.1 Slice (prelude)

| Signature | Description |
|-----------|-------------|
| `as_slice(source: Vec<T>, start: i32, end: i32) -> Slice<T>` | Create a bounded, non-`Copy` read view without consuming the source binding |
| `slice_len(view: Slice<T>) -> i32` | Consume the view and return its bounded length |
| `slice_get(view: Slice<T>, index: i32) -> T` | Consume the view and read an element within the bounded range |

The current view API does not provide full lifetime or mutation-exclusion
checking. Those rules are separate from the owned ADT contract and are tracked
by the subsequent borrow/closure work.

### 9.13 Assertions (prelude)

| Signature | Description |
|-----------|-------------|
| `assert(cond: bool)` | Assert true |
| `assert_eq(a: i32, b: i32)` | Assert equal (i32) |
| `assert_ne(a: i32, b: i32)` | Assert not equal (i32) |
| `assert_eq_i64(a: i64, b: i64)` | Assert equal (i64) |
| `assert_eq_str(a: String, b: String)` | Assert equal (String) |

### 9.14 HashMap (generic public API)

| Signature | Description |
|-----------|-------------|
| `HashMap::new<K, V>() -> HashMap<K, V>` | Construct a statically monomorphized map |
| `m.insert(k: K, v: V) -> Option<V>` | Insert or update and return the previous value |
| `m.get(k: K) -> Option<V>` | Lookup |
| `m.contains_key(k: K) -> bool` | Key test |
| `m.remove(k: K) -> Option<V>` | Remove and return the previous value |
| `m.len() -> i32` | Number of entries |
| `m.is_empty() -> bool` | Whether the map has no entries |
| `m.keys() -> Vec<K>` | Eager snapshot of all keys |
| `m.values() -> Vec<V>` | Eager snapshot of all values |
| `m.entries() -> Vec<(K, V)>` | Eager snapshot of all key/value pairs |

The current concrete adapter set supports `i32` and `String` keys with `i32`, `String`,
or `fn(i32) -> i32` values, for the six key/value pairs listed in
[ADR-066](../adr/ADR-066-language-v2-generic-public-api.md). A concrete pair outside that
set is a typecheck error; it is not erased to `Any`.

`keys()`, `values()`, and `entries()` materialize a `Vec` snapshot of the pairs
present at the call: inserts or removals performed afterwards do not appear in an
already returned vec. Element order inside the snapshot follows the internal
bucket order and is implementation-defined; programs must not depend on it
(identical runs may permute). Iterate the returned `Vec` positionally, e.g.
`let (k, v) = es.get_unchecked(i)` inside a `while` loop, or destructure each
pair after fetching it.

The compiler statically monomorphizes the same source API for `wasm32-gc` and the `wasm32`
linear-memory fallback. Existing type-suffixed builtins remain compatibility entries during
the ADR-014 migration window and are not the recommended source form.

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

**Time/Duration** (`std::time`):
`Duration::between(start: i64, end: i64)`; methods `d.as_ms()`, `d.as_us()`,
`d.as_ns()`

**Hash**:
`h1.combine_hash(h2)` on the first hash value; the canonical hash surface is
the `Hash` trait method `value.hash()`

**Seeded Random** (`std::random`):
`Rng::new(seed)`; methods `rng.next()`, `rng.range(lo, hi)`,
`rng.shuffle(v)`

**Range**:
`Range::new(start: i32, end: i32)` and
`RangeInclusive::new(start: i32, end: i32)`; methods `r.contains(x)` and
`r.len()` on `Range`, `r.contains(x)` on `RangeInclusive`

**Error**:
`e.message()` on the `Error` value

**Path** (`std::path`):
`Path::new(path)`; methods `p.extension()`, `p.file_name()`, `p.parent()`,
`p.is_absolute()`, `p.with_extension(ext)`, `p.join(segment)`, `p.stem()`,
`p.normalize()`, `p.components()`, `p.into_string()`

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

**Bytes** (`std::bytes`; `Bytes` = `Vec<i32>`):
methods `b.push_byte(x)`, `b.len()`, `b.get_unchecked(i)`,
`b.equals_bytes(other)`, `b.concat_bytes(other)`, `b.slice_bytes(start, end)`,
`b.to_hex()`, `b.to_base64()`, `b.push_u16_le(x)`, `b.push_u32_le(x)`,
`b.push_u64_le(x)`, `b.extend_bytes(other)`, `b.freeze()`,
`data.byte_cursor()`, `c.cursor_pos()`, `c.cursor_remaining()`,
`c.read_u8()` / `read_u16_le` / `read_u32_le` / `read_u64_le` /
`read_u32_be` / `read_bytes(n)`, the `c.try_read_*` `Option` readers, and
`c.read_leb128_u32` / `read_leb128_i32` / `read_leb128_u64`;
associated `Bytes::from_array(xs)`, `Bytes::from_hex(s)`,
`Bytes::from_base64(s)`, `Bytes::from_u16_le(x)`, `Bytes::from_u32_le(x)`,
`Bytes::from_u32_be(x)`, `Bytes::from_leb128_u32(x)`,
`Bytes::from_leb128_i32(x)`, `Bytes::from_leb128_u64(x)`.
Import `use std::bytes` for the methods and `use std::bytes::Bytes` for the
associated functions. The 45 registered free functions (`bytes_new`,
`bytes_push`, `hex_encode`, `cursor_new`, …) were removed in the ADR-087
Wave 2 deletion; qualified old calls fail resolution with an E0100 hint that
names the replacement.

**Endian Conversion**:
`Bytes::from_u16_le(x)` / `b.to_u16_le()`,
`Bytes::from_u32_le(x)` / `b.to_u32_le()`,
`Bytes::from_u32_be(x)` / `b.to_u32_be()`

**Linear Memory** (`wasm32` target / linear lowering helpers):
bulk `memory.copy` / `memory.fill` are compiler intrinsics only
(`__intrinsic_memory_copy`, `__intrinsic_memory_fill`); the experimental
`std::wasm` free wrappers were removed under ADR-046.

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

**JSON**:
primitive helpers `json_null()`, `json_parse_i32(s)` / `json_parse_bool(s)`,
`json_stringify_i32(n)` / `json_stringify_bool(b)` /
`json_stringify_string(s)`; the DOM scanner free helpers (`skip_ws`,
`quote_string`, `json_escape`, `json_get_str`, …); and the `JsonValue`
value methods `v.is_null()` / `v.is_bool()` / `v.is_number()` /
`v.is_string()` / `v.is_array()` / `v.is_object()`, `v.as_bool()` /
`v.as_string()` / `v.as_i32()` / `v.as_f64()` / `v.as_array()`,
`v.get_field(key)` / `v.get_index(i)`, `v.to_text()`, plus
`e.message()` on `JsonParseError`. `json::parse(doc)` remains the document
entry point.

**CSV**:
`csv_split_line(line: String) -> Vec<String>` and the row/document helpers
(`csv_parse_row`, `csv_stringify_row`, `csv_count_rows`, `csv_get_row_raw`,
`csv_parse_row_at`, `csv_parse`, `csv_stringify`, `csv_parse_with_header`).

**TOML**:
`toml_parse_line(line)`, the `find_toml_section` / `find_toml_value` /
`toml_find_raw_value` / `toml_parse_string_array` scanners, and the
`TomlValue` methods `v.as_string()` / `v.as_int()` / `v.as_bool()` /
`v.as_float()` / `v.as_array()` / `v.as_table()`, `v.get_field(key)`,
`v.table_keys()`. `toml::toml_parse(doc)` and `toml::toml_stringify(v)`
remain the document entry points.

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
`Component::canonical_abi_version() -> i32`,
`Component::model_version() -> String`

**WIT Type Helpers**:
`WitType` (enum: `Bool`, `U8`, `U16`, `U32`, `U64`, `S8`, `S16`, `S32`,
`S64`, `F32`, `F64`, `Char`, `String`, `List`, `Option`, …),
`WitType::list(inner: WitType) -> WitType`,
`WitType::option(inner: WitType) -> WitType`,
`(ty).id() -> i32`, `WitType::from_id(id: i32) -> WitType`,
`(ty).name() -> String`, `(ty).wit_text() -> String`,
`WitType::name_from_ark(name: String) -> String`

---

## 10. Error Codes <!-- stability: stable -->

The type checker emits diagnostics with the following error codes:

| Code | Description |
|------|-------------|
| E0200 | Type mismatch |
| E0202 | Undefined variable or function |
| E0205 | Incompatible types in binary operation |
| E0207 | Incorrect number of function arguments |
| E0213 | Borrowed closure capture escapes its enclosing function |
| E0214 | Closure capture conflicts with another borrow or move |

Additional error codes are implementation-defined and may be added
by the compiler without an ADR.

---

## Appendix A. Grammar Summary (EBNF)

```ebnf
module      = doc_comment* import_decl* item* ;
import_decl = legacy_import | wit_import | use_decl ;
legacy_import
            = "import" IDENT ("as" IDENT)? ;
wit_import  = "import" STRING ("as" IDENT)? ;
use_decl   = "pub"? "use" use_path ("as" IDENT)?
              | "pub"? "use" use_path "::" "{" use_item_list "}" ;
use_path   = IDENT ("::" IDENT)* ;
use_item_list
            = IDENT ("," IDENT)* ","? ;
item        = fn_def | fn_clause_group
            | ("pub")? (struct_def | enum_def | trait_def | impl_block) ;

fn_prefix   = "pub"? "async"? "export"? ;
fn_def      = fn_prefix "fn" IDENT type_params? "(" param_list? ")"
              ("->" type_expr)? block ;
              (* unchanged -- single body form *)

fn_clause_group
            = fn_clause (NEWLINE+ fn_clause)* where_block? ;
              (* every repeated clause has the same name and arity *)
              (* a semicolon terminates the group *)

fn_clause   = fn_prefix "fn" IDENT type_params? "(" clause_params? ")"
              ("->" type_expr)? ("|" expr)? "=" expr ;
              (* mandatory `= expr` body -- no block form *)
              (* optional guard via `| expr` before `=` *)

clause_params = clause_param ("," clause_param)* ","? ;
clause_param  = pattern ;
              (* patterns instead of typed identifiers *)
struct_def  = "struct" IDENT type_params? "{" field_list? "}" ;
enum_def    = "enum" IDENT type_params? "{" variant_list? "}" ;
trait_def   = "trait" IDENT type_params? "{" trait_item* "}" ;
impl_block  = "impl" type_params? type_expr ("for" type_expr)? "{" impl_item* "}" ;
              (* impl type parameters come only from type_params (ADR-089) *)
trait_item  = associated_type_decl | method_sig ;
impl_item   = associated_type_def | fn_def ;
associated_type_decl
            = "type" IDENT ("=" type_expr)? ;
associated_type_def
            = "type" IDENT "=" type_expr ;

type_params = "<" type_param ("," type_param)* ">" ;
type_param  = IDENT (":" IDENT ("+" IDENT)*)? ;
param_list  = param ("," param)* ","? ;
param       = "mut"? IDENT ":" type_expr ;
              (* "mut": mutable borrow, ADR-088 §3.5.1; not yet parsed *)
field_list  = field ("," field)* ","? ;
field       = IDENT ":" type_expr ;
variant_list= variant ("," variant)* ","? ;
variant     = IDENT
            | IDENT "(" type_expr ("," type_expr)* ")"
            | IDENT "{" field_list? "}" ;
method_sig  = "fn" IDENT "(" param_list? ")" ("->" type_expr)? ;
where_block = "where" where_binding (where_separator where_binding)*
              where_trailing_separator? ;
where_binding = IDENT "=" expr ;
where_separator = NEWLINE+ | ";" ;
where_trailing_separator = NEWLINE* ";" ;

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
            | "[" (array_init | comprehension) "]"
            | IDENT "{" struct_fields "}"
            | "if" expr block ("else" (if_expr | block))?
            | "match" expr "{" match_arm* "}"
            | "loop" block
            | "|" closure_params? "|" ("->" type_expr)? expr
            | "move" "|" closure_params? "|" ("->" type_expr)? expr
            | "return" expr?
            | "break" expr?
            | "continue" ;

array_init  = expr (";" expr)?
            | expr ("," expr)* ;

comprehension = expr "for" IDENT "in" expr ("if" expr)? ;
struct_fields = (IDENT ":" expr) ("," IDENT ":" expr)*
                ("," ".." expr)? ;
match_arm   = pattern ("if" expr)? "=>" expr ","? ;
closure_params = closure_param ("," closure_param)* ;
closure_param  = "mut"? IDENT (":" type_expr)? ;
                 (* "mut": ADR-088 §3.5.1; not yet parsed *)

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
            | "dyn" type_expr
            | IDENT
            | IDENT "." IDENT
            | IDENT "<" type_list ">" ;
type_list   = type_expr ("," type_expr)* ;
expr_list   = expr ("," expr)* ;
```

---

## Appendix B. Compilation Targets

Per [ADR-007](../adr/ADR-007-targets.md) / [ADR-013](../adr/ADR-013-primary-target.md),
canonical compilation targets are:

| Target | Tier | Notes |
|--------|------|-------|
| `wasm32-gc` | **primary** | Wasm GC value representation. Default host profile = WASI P2. Component emit (ADR-054). |
| `wasm32` | supported | Same language semantics, linear-memory lowering. AtCoder / non-GC compatibility. |
| `native-cpp` | scaffold | Experimental `run --target native-cpp` and `compile --emit c` (ADR-049 / ADR-050); implementation partial. |
| `native-llvm` | scaffold | Experimental. ABI and semantics remain undecided. |

Retired / not public contracts:

- `wasm32-freestanding` — hard error as a public target name (ADR-007)
- Old labels T1–T5 — historical only; do not use in new docs

Legacy CLI aliases (`wasm32-wasi-p1` → `wasm32`, `wasm32-wasi-p2` → `wasm32-gc`, …)
are defined in ADR-007; implementation migration status is in `docs/current-state.md`.

---

## Appendix C. Design Decision References

| ADR | Decision |
|-----|----------|
| ADR-002 | Wasm GC adopted as language memory semantics; `wasm32` is linear lowering |
| ADR-003 | Monomorphisation for generics; no language-level limit on parameters or nesting |
| ADR-006 | Public ABI categories: compiler-private / stable WIT-canonical / experimental raw Wasm / reserved native |
| ADR-007 | Canonical targets `wasm32` / `wasm32-gc` / `native-*` (T1–T5 retired) |
| ADR-054 | compiler emits standard WASI P2 core/WIT inputs; official `wasm-tools` packages components; bridge and legacy ABI paths are deleted |
| ADR-009 | `use` for source imports; `import` reserved for WIT |
| ADR-011 | Host-bound APIs in `std::host::*`; pure APIs in `std::*` |
| ADR-013 | Primary target = `wasm32-gc` (default host WASI P2) |
| ADR-031 | Import / WIT package syntax unification (supersedes ADR-025/026 exploration) |
| ADR-071 | `pub` と Component export、WIT identity の境界 |
| ADR-044 | Trait / method syntax adopted (supersedes ADR-004) |
| ADR-045 | Native/LLVM decision deferral, superseded by ADR-049 for the limited native-cpp executor scope |
| ADR-049 | Native C99 selfhost executor with a compiler-private ABI; current implementation remains scaffold |
| ADR-046 | Free-function eradication: public API is trait / method / associated function |
| ADR-059 | Ownership, `Copy`, and mutability of bindings (`let` moves, explicit `impl Copy`, `let mut`) |
| ADR-060 | Owned ADTs, unique-owner `Box<T>`, read-only `Slice<T>` view |
| ADR-061 / ADR-081 / ADR-083 | Closure capture modes, escape via promoted shared cells, nested capture-through |
| ADR-062 / ADR-078 | Trait core and coherence; parametric (non-blanket) trait impls |
| ADR-063 / ADR-064 / ADR-084 | `?` conversion boundary, `Error` trait, explicit fallible conversion |
| ADR-065 | Operator syntax resolves through canonical operator traits |
| ADR-066 | Generic public API (e.g. `HashMap<K, V>`) |
| ADR-067 / ADR-068 | No implicit numeric conversion; arithmetic evaluation semantics |
| ADR-069 | String UTF-8 and Unicode scalar semantics |
| ADR-070 | Let-generalization with value restriction |
| ADR-072 / ADR-073 | Fixed Prelude and keyword policy; multi-clause / `where` / semicolon grammar |
| ADR-074 | Target capability configuration |
| ADR-076 / ADR-077 | Iterator consumer boundary; hygienic desugaring |
| ADR-079 / ADR-080 | Method-level generics; generic element storage policy |
| ADR-082 | Match exhaustiveness is a compile-time requirement |
| ADR-085 / ADR-086 | I/O trait unification; linear collection ADTs |
| ADR-088 | Call boundary: implicit borrow, `mut` parameters, mutable places |
| ADR-089 | Impl type parameters only via explicit `impl<…>` binders |
