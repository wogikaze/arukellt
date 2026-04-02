# Error Code Reference

> Canonical listing of every diagnostic code emitted by the Arukellt compiler.
> Source of truth: `crates/ark-diagnostics/src/codes.rs`.

## Overview

Diagnostic codes follow the pattern `E0NNN` (errors) or `W0NNN` (warnings).
The first digit after the prefix identifies the compiler phase:

| Range | Phase | Crate(s) |
|-------|-------|----------|
| `E000x` | Parse / Lex | `ark-lexer`, `ark-parser` |
| `E01xx` | Name resolution | `ark-resolve` |
| `E02xx` | Type checking | `ark-typecheck` |
| `E03xx` | v0 version constraints / target | `ark-typecheck`, target selection |
| `E04xx` | Component Model / canonical ABI | backend validate |
| `W0xxx` | Warnings & validation gate | various |
| `ICE-*` | Internal compiler errors | runtime |

Severity is one of **error**, **warning**, or **help**.

---

## Parse Errors — `E000x`

### E0001 — unexpected token

| | |
|---|---|
| **Severity** | error |
| **Phase** | parse |
| **Message** | unexpected token |

Emitted when the parser encounters a token it does not expect at the current
position.

```arukellt
fn main() {
    let x = ;
}
```

### E0002 — missing token

| | |
|---|---|
| **Severity** | error |
| **Phase** | parse |
| **Message** | missing token |

Emitted when a required token (e.g. closing brace or semicolon) is absent.

```arukellt
fn main() {
    let x = 42

```

### E0003 — invalid construct

| | |
|---|---|
| **Severity** | error |
| **Phase** | parse |
| **Message** | invalid construct |

Catch-all for lexer-level invalid constructs: unterminated strings, invalid
float/integer literals, unterminated block comments, bad escape sequences, etc.

```arukellt
fn main() {
    let s = "unterminated
}
```

---

## Name Resolution Errors — `E01xx`

### E0100 — unresolved name

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | unresolved name |

A symbol is used but has no definition in scope.

```arukellt
fn main() {
    let x = unknown_function()
}
```

### E0101 — duplicate definition

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | duplicate definition |

Two definitions with the same name exist in the same scope.

```arukellt
fn foo() -> i32 {
    1
}

fn foo() -> i32 {
    2
}

fn main() {
    foo()
}
```

### E0102 — access to private symbol

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | access to private symbol |

Code references a symbol that is not exported from its defining module.

### E0103 — circular import

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | circular import |

Two or more modules import each other, forming a cycle.

### E0104 — module not found

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | module not found |

An `use` declaration references a module path that does not exist. This
commonly occurs when a standard library module has been relocated.

```arukellt
use std::io

fn main() {}
```

> **Fix:** `std::io` moved to `std::host::stdio`.

---

## Type Checking Errors — `E02xx`

### E0200 — type mismatch

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | type mismatch |

The most common type error. Emitted when the expected type does not match the
actual type in assignments, return values, function arguments, etc.

```arukellt
fn main() {
    let x: i32 = "hello"
}
```

### E0201 — missing type annotation

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | missing type annotation |

A function parameter or binding lacks a required type annotation.

```arukellt
fn add(a, b) -> i32 {
    a + b
}

fn main() {
    add(1, 2)
}
```

### E0202 — wrong number of arguments

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | wrong number of arguments |

A function call supplies more or fewer arguments than the function signature
requires.

```arukellt
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    add(1, 2, 3)
}
```

### E0203 — invalid generic usage

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | invalid generic usage |

Generic type parameters are used incorrectly (e.g. wrong arity or invalid
constraint).

### E0204 — non-exhaustive match

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | non-exhaustive match |

A `match` expression does not cover all variants of the scrutinee type.

```arukellt
use std::host::stdio
enum Color {
    Red,
    Green,
    Blue,
}

fn main() {
    let c: Color = Color::Red
    match c {
        Color::Red => stdio::println("red"),
        Color::Green => stdio::println("green"),
    }
}
```

### E0205 — mismatched match arm types

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | mismatched match arm types |

The arms of a `match` expression produce values of different types.

```arukellt
fn main() {
    let x: i32 = 1
    let y = match x {
        1 => 42,
        _ => "hello",
    }
}
```

### E0206 — invalid pattern

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | invalid pattern |

A pattern in a `match` arm is syntactically or semantically invalid.

### E0207 — cannot mutate immutable variable

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | cannot mutate immutable variable |

An assignment targets a variable declared without `mut`.

```arukellt
fn main() {
    let x: i32 = 10
    x = 20
}
```

### E0208 — missing return value

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | missing return value |

A function with a non-unit return type does not produce a value on all paths.

### E0209 — unreachable pattern

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | unreachable pattern |

A pattern in a `match` expression can never be reached because earlier arms
already cover it.

### E0210 — incompatible error type for `?` operator

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | incompatible error type for `?` operator |

The `?` operator is used in a function whose return type is not `Result`, or
the error type of the inner expression is incompatible with the outer return
type.

```arukellt
fn foo() -> i32 {
    let r: Result<i32, String> = Ok(42)
    let val = r?
    val
}

fn main() {
    foo()
}
```

---

### E0211 — module contains only unimplemented host stubs

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | module contains only unimplemented host stubs |

Importing a host module that is entirely composed of unimplemented stubs
(e.g., `std::host::http` on a target that does not support WASI Preview 2)
produces this error. The module exists in the manifest but has no callable
implementations for the current target.

---

## v0 Version Constraints & Target Errors — `E03xx`

These errors enforce the v0 language subset and target compatibility.

### E0300 — traits are not available in v0

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | traits are not available in v0 |

Trait definitions are not supported in the current language version.

```arukellt
trait Display {
    fn display(self) -> String
}

fn main() {
}
```

### E0301 — method call syntax is not available in v0

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | method call syntax is not available in v0 |

Dot-method call syntax is not supported in v0; use free functions instead.

```arukellt
fn main() {
    let mut v: Vec<i32> = Vec_new_i32()
    v.push(42)
}
```

### E0302 — nested generics are not allowed in v0

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | nested generics are not allowed in v0 |

Generic types may not be nested (e.g. `Vec<Vec<i32>>`) in v0.

```arukellt
fn main() {
    let v: Vec<Vec<i32>> = Vec_new_i32()
}
```

### E0303 — `for` loop is not available in v0

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | `for` loop is not available in v0 |

`for` loops are not part of the v0 language subset.

### E0304 — operator overloading is not available in v0

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | operator overloading is not available in v0 |

`impl` blocks for operator traits are not supported in v0.

```arukellt
impl Add for Point {
    fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x }
    }
}

fn main() {
}
```

### E0305 — unsupported target

| | |
|---|---|
| **Severity** | error |
| **Phase** | target |
| **Message** | unsupported target |

The requested compilation target is not recognized.

### E0306 — invalid emit kind for target

| | |
|---|---|
| **Severity** | error |
| **Phase** | target |
| **Message** | invalid emit kind for target |

The specified emit kind (e.g. object, library) is not valid for the selected
target.

### E0307 — feature not available for target

| | |
|---|---|
| **Severity** | error |
| **Phase** | target |
| **Message** | feature not available for target |

A language or runtime feature required by the program is not available on the
selected compilation target.

---

## Component Model / Canonical ABI Errors — `E04xx`

These errors are emitted during backend validation when the program uses
Component Model features that are not yet implemented.

### E0400 — WIT flags type not supported

| | |
|---|---|
| **Severity** | error |
| **Phase** | backend-validate |
| **Message** | WIT flags type not supported in current version |

The exported function uses a WIT `flags` type which is not yet supported.

### E0401 — canonical ABI not implemented for compound type

| | |
|---|---|
| **Severity** | error |
| **Phase** | backend-validate |
| **Message** | canonical ABI not implemented for compound type in component export |

A component export uses a compound type (string, list, tuple, record, etc.)
whose canonical ABI lowering/lifting is not yet implemented.

### E0402 — WIT resource type not implemented

| | |
|---|---|
| **Severity** | error |
| **Phase** | backend-validate |
| **Message** | WIT resource type not implemented in current version |

The exported function uses a WIT `resource` type which is not yet implemented.

---

## Warnings — `W0xxx`

### W0001 — possible unintended sharing of reference type

| | |
|---|---|
| **Severity** | warning |
| **Phase** | typecheck |
| **Message** | possible unintended sharing of reference type |

Fires when a mutable reference type is aliased within the same function body,
which may lead to unexpected shared mutation.

```arukellt
fn main() {
    let mut v: Vec<i32> = Vec_new_i32()
    push(v, 1)
    let alias = v
    push(v, 2)
}
```

### W0002 — deprecated target alias

| | |
|---|---|
| **Severity** | warning |
| **Phase** | target |
| **Message** | deprecated target alias |

The specified target name is a deprecated alias. Use the canonical target name
instead.

### W0003 — ambiguous import

| | |
|---|---|
| **Severity** | warning |
| **Phase** | resolve |
| **Message** | ambiguous import: local and std modules share the same name |

A local module and a standard library module share the same name, making the
import ambiguous.

### W0004 — generated Wasm module failed validation

| | |
|---|---|
| **Severity** | **error** (promoted from warning) |
| **Phase** | backend-validate |
| **Message** | generated Wasm module failed validation |

The generated Wasm binary fails `wasmparser::Validator::validate_all()`.
Despite the `W` prefix, this is treated as a **hard error** — invalid Wasm
is never accepted as a successful build.

### W0005 — non-exportable function skipped

| | |
|---|---|
| **Severity** | warning |
| **Phase** | backend-validate |
| **Message** | function has non-exportable parameter type, skipped from component exports |

A public function uses parameter types that cannot be lowered through the
canonical ABI and is therefore omitted from the component's export list.

---

### W0006 — unused import

| | |
|---|---|
| **Severity** | warning |
| **Phase** | resolve |
| **Message** | unused import `<module>` |

An imported module is never referenced via qualified identifiers in the
current file. Remove the import or prefix the alias with `_` to suppress
this warning.

```ark
use std::host::fs       // W0006 if no fs:: usage
use std::math as _math  // suppressed by _ prefix
```

**Fix-it**: available as a quick-fix code action in the editor.

---

### W0007 — unused binding

| | |
|---|---|
| **Severity** | warning |
| **Phase** | typecheck |
| **Message** | unused binding `<name>` |

A `let` binding introduces a name that is never referenced in the enclosing
function. Remove the binding or prefix the name with `_` to suppress this
warning.

```ark
let _unused = expensive()  // suppressed by _ prefix
let x = 1                  // W0007 if x is never used
```

---

### W0008 — deprecated API

| | |
|---|---|
| **Severity** | warning |
| **Phase** | resolve |
| **Message** | deprecated API: use `<replacement>` instead |

A function marked as deprecated in `std/manifest.toml` is being used.
The `deprecated_by` field indicates the recommended replacement.

```ark
// Vec_new_i32() is deprecated — use Vec::new() with type annotation
let v = Vec_new_i32()  // W0008: deprecated API
```

---

## Internal Compiler Errors — `ICE-*`

These are not language errors but signals of compiler bugs. If you encounter
one, please file a bug report.

| Code | Phase | Meaning |
|------|-------|---------|
| `ICE-PIPELINE` | internal | Unexpected failure in the compilation pipeline |
| `ICE-MIR` | internal | Bug in MIR construction or optimization |
| `ICE-BACKEND` | internal | Bug in the code generation backend |

---

## Quick Reference Table

| Code | Severity | Phase | Message |
|------|----------|-------|---------|
| E0001 | error | parse | unexpected token |
| E0002 | error | parse | missing token |
| E0003 | error | parse | invalid construct |
| E0100 | error | resolve | unresolved name |
| E0101 | error | resolve | duplicate definition |
| E0102 | error | resolve | access to private symbol |
| E0103 | error | resolve | circular import |
| E0104 | error | resolve | module not found |
| E0200 | error | typecheck | type mismatch |
| E0201 | error | typecheck | missing type annotation |
| E0202 | error | typecheck | wrong number of arguments |
| E0203 | error | typecheck | invalid generic usage |
| E0204 | error | typecheck | non-exhaustive match |
| E0205 | error | typecheck | mismatched match arm types |
| E0206 | error | typecheck | invalid pattern |
| E0207 | error | typecheck | cannot mutate immutable variable |
| E0208 | error | typecheck | missing return value |
| E0209 | error | typecheck | unreachable pattern |
| E0210 | error | typecheck | incompatible error type for `?` operator |
| E0211 | error | resolve | module contains only unimplemented host stubs |
| E0300 | error | typecheck | traits are not available in v0 |
| E0301 | error | typecheck | method call syntax is not available in v0 |
| E0302 | error | typecheck | nested generics are not allowed in v0 |
| E0303 | error | typecheck | `for` loop is not available in v0 |
| E0304 | error | typecheck | operator overloading is not available in v0 |
| E0305 | error | target | unsupported target |
| E0306 | error | target | invalid emit kind for target |
| E0307 | error | target | feature not available for target |
| E0400 | error | backend-validate | WIT flags type not supported in current version |
| E0401 | error | backend-validate | canonical ABI not implemented for compound type |
| E0402 | error | backend-validate | WIT resource type not implemented in current version |
| W0001 | warning | typecheck | possible unintended sharing of reference type |
| W0002 | warning | target | deprecated target alias |
| W0003 | warning | resolve | ambiguous import |
| W0004 | **error** | backend-validate | generated Wasm module failed validation |
| W0005 | warning | backend-validate | non-exportable function skipped |
| W0006 | warning | resolve | unused import |
| W0007 | warning | typecheck | unused binding |
| W0008 | warning | resolve | deprecated API |

---

## Diagnostic Output Format

The compiler renders diagnostics in the following format:

```
error[E0200|typecheck]: type mismatch
  --> src/main.ark:2:17
   |
2  |     let x: i32 = "hello"
   |                   ^^^^^^^ expected i32, actual String
```

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `ARUKELLT_DUMP_DIAGNOSTICS=1` | Emit structured diagnostics snapshot |
| `ARUKELLT_DUMP_PHASES=parse,resolve,mir` | Dump intermediate phase output |

## Update Policy

This document is maintained manually. When adding or modifying diagnostic codes
in `crates/ark-diagnostics/src/codes.rs`, update this file to match.
`scripts/check/check-docs-consistency.py` includes a check for error-code-to-doc
alignment.

## See Also

- [diagnostics.md](diagnostics.md) — diagnostic system architecture
- [pipeline.md](pipeline.md) — compiler pipeline overview
- [../language/error-handling.md](../language/error-handling.md) — language-level error handling
