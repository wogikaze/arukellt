# Error Code Reference

> Declared diagnostic code catalogue, including implemented and reserved codes.
> Code declarations: `src/compiler/diagnostics/codes.ark`.
> Lifecycle identity: `docs/data/warnings.toml`.

## Overview

Diagnostic codes follow the pattern `E0NNN` (errors) or `W0NNN` (warnings).
The first digit after the prefix identifies the compiler phase:

| Range | Phase | Crate(s) |
|-------|-------|----------|
| `E000x` | Parse / Lex | `src/compiler/lexer.ark`, `src/compiler/parser.ark` |
| `E01xx` | Name resolution | `src/compiler/resolver.ark` |
| `E02xx` | Type checking | `src/compiler/typechecker.ark` |
| `E03xx` | v0 version constraints / target | `src/compiler/typechecker.ark`, target selection |
| `E04xx` | Component Model / canonical ABI | backend validate |
| `E05xx` | Resolve / target gating | `src/compiler/resolver.ark` |
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

### E0004 — expected token

| | |
|---|---|
| **Severity** | error |
| **Phase** | parse |
| **Message** | expected token |

Emitted when parser recovery expected a specific token kind at the current
position.

### E0090 — WIT flags type is not supported in v2

| | |
|---|---|
| **Severity** | error |
| **Phase** | backend-validate |
| **Message** | WIT flags type is not supported in v2; use individual bool parameters instead |

Emitted when component-model validation encounters a WIT `flags { ... }` type
in a surface that only supports lowerable primitive and aggregate types.

```wit
interface example {
    flags permissions { read, write }
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

### E0120 — registry unreachable

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | registry unreachable |

The registry endpoint was not reachable (network error or timeout). In v1, only
local file-based mock registries (`file://./path`) are supported; HTTP(S)
endpoints emit this error.

### E0121 — package not found in registry

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | package not found in registry |

The declared package was not found in the configured registry. Occurs when the
local mock directory (or network registry) does not contain the requested
package.

```toml
# ark.toml — package declared but not present in registry
[dependencies]
my-lib = "1.0.0"
```

### E0122 — version not found in registry

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | version not found in registry |

The package exists in the registry but the requested version is not available.

### E0123 — integrity check failed

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | integrity check failed for downloaded package |

The SHA-256 checksum of the downloaded package archive does not match the
checksum recorded in the registry manifest.

### E0124 — no registry configured

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | no registry configured |

A registry dependency was declared in `[dependencies]` but no `[registry]`
section exists in `ark.toml`.

```toml
# ark.toml — add this section to fix E0124
[registry]
url = "file://./mock_reg"   # local mock
# url = "https://registry.arukellt.dev/v1"  # public registry (planned)
```

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

`stream<T>` and `future<T>` async WIT types are accepted for WIT import parsing as of #474 Phase 4, but full async component export lowering is deferred to a later phase of #474.

---

## Resolve Errors — `E05xx`

### E0500 — module requires a different target

| | |
|---|---|
| **Severity** | error |
| **Phase** | resolve |
| **Message** | module requires a different target |

A stdlib module was imported that is only available on a specific target
(e.g., `std::host::http` on a target that does not support host networking).
Switch to a target that supports the module, or remove the import.

---

### E0501 — symbol not found in module

| | |
|---|---|
| **Severity** | error |
| **Phase** | typecheck |
| **Message** | symbol not found in module |

A qualified call references a function or symbol that does not exist in the
imported module. For example, `string::nonexistent(...)` will trigger this
error if `nonexistent` is not a public function in `std::text::string`.
Check the spelling or verify the function is exported by the module.

---

## Warnings — `W0xxx`

### W0001 — possible unintended sharing of reference type

| | |
|---|---|
| **Severity** | warning |
| **Phase** | typecheck |
| **Maturity** | implemented; emitted |
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
| **Phase** | lint-post-resolve |
| **Maturity** | implemented; emitted |
| **Message** | deprecated target alias |

The specified target name is a deprecated alias. Use the canonical target name
instead.

### W0003 — unused symbol

| | |
|---|---|
| **Severity** | warning |
| **Phase** | lint-post-resolve |
| **Maturity** | implemented; emitted |
| **Message** | unused symbol `<name>` |

A declared symbol is not referenced. Remove it or apply the supported
underscore convention when the unused declaration is intentional.

### W0004 — generated Wasm module failed validation

| | |
|---|---|
| **Severity** | error |
| **Phase** | backend-validate |
| **Maturity** | implemented; emitted |
| **Message** | generated Wasm module failed validation |

The generated Wasm binary fails `wasmparser::Validator::validate_all()`.
Despite the `W` prefix, this is treated as a **hard error** — invalid Wasm
is never accepted as a successful build.

### W0005 — non-exportable function skipped

| | |
|---|---|
| **Severity** | warning |
| **Phase** | component |
| **Maturity** | implemented; emitted |
| **Message** | function has non-exportable parameter type, skipped from component exports |

A public function uses parameter types that cannot be lowered through the
canonical ABI and is therefore omitted from the component's export list.

---

### W0006 — unused import

| | |
|---|---|
| **Severity** | warning |
| **Phase** | lint-post-resolve |
| **Maturity** | implemented; emitted |
| **Message** | unused import `<module>` |

An imported module is never referenced via qualified identifiers in the
current file. Remove the import or prefix the alias with `_` to suppress
this warning.

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
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
| **Phase** | lint-post-resolve |
| **Maturity** | implemented; emitted |
| **Message** | unused binding `<name>` |

A `let` binding introduces a name that is never referenced in the enclosing
function. Remove the binding or prefix the name with `_` to suppress this
warning.

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let _unused = expensive()  // suppressed by _ prefix
let x = 1                  // W0007 if x is never used
```

---

### W0008 — documentation drift

<a id="w0008-documentation-drift"></a>

| | |
|---|---|
| **Severity** | warning |
| **Phase** | lint-post-resolve |
| **Message** | documentation drift |
| **Maturity** | declared; not currently emitted |

Reserved for documentation drift detected from tracked documentation comments.
The current compiler declares and registers this code, but does not emit it
because full comment tracking is not implemented.

---

### W0009 — deprecated API

<a id="w0009-deprecated-api"></a>

| | |
|---|---|
| **Severity** | warning |
| **Phase** | lint-post-resolve |
| **Maturity** | implemented; emitted |
| **Message** | deprecated API: use `<replacement>` instead |

A function marked as deprecated in the compiler's generated deprecation table
is being used. The manifest `deprecated_by` field records the intended
replacement; it does not by itself prove current callability.

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
// Vec_new_i32() is deprecated — use Vec::new() with type annotation
let v = Vec_new_i32()  // W0009: deprecated API
```

---

### W0010 — prefer use import

<a id="w0010-prefer-use-import"></a>

| | |
|---|---|
| **Severity** | warning |
| **Phase** | lint-post-resolve |
| **Maturity** | implemented; emitted |
| **Message** | qualified path `<path>` used repeatedly; prefer `use` and call the short name |

A module-style qualified path (`snake_case::symbol`) appears at least three
times in one file. Prefer importing the symbol with `use` (including
function-level destructuring such as `use path::{symbol}`) and calling the
short name. Type or enum qualifiers (`PascalCase::...`) are not reported.

Evidence fixture: `tests/fixtures/diagnostics/prefer_use_import.ark`.

**Suppress**: `--allow W0010` or `--allow prefer-use-import`, or reduce
qualified uses below the threshold.

---

### W0011 — prefer else-if

<a id="w0011-prefer-else-if"></a>

| | |
|---|---|
| **Severity** | warning |
| **Phase** | lint-post-resolve |
| **Maturity** | implemented; emitted |
| **Message** | nested `else { if ... }` can be written as `else if`; prefer else-if chaining |

An `if` expression whose else branch is a block whose only statement is another
`if` can be rewritten with `else if`. True `else if` is represented in the AST
as an `IfExpr` child; the nested-block form is the anti-pattern this rule
flags.

Evidence fixture: `tests/fixtures/diagnostics/prefer_else_if.ark`.

**Suppress**: `--allow W0011` or `--allow prefer-else-if`, or rewrite the
chain with `else if`.

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
| E0090 | error | backend-validate | WIT flags type is not supported in v2; use individual bool parameters instead |
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
| E0500 | error | resolve | module requires a different target |
| E0501 | error | typecheck | symbol not found in module |
<!-- BEGIN GENERATED:WARNING_CODE_CATALOGUE -->
| W0001 | warning | typecheck | possible unintended sharing of reference type |
| W0002 | warning | lint-post-resolve | deprecated target alias |
| W0003 | warning | lint-post-resolve | unused symbol |
| W0004 | **error** | backend-validate | generated Wasm failed validation |
| W0005 | warning | component | non-exportable function skipped from component exports |
| W0006 | warning | lint-post-resolve | unused import |
| W0007 | warning | lint-post-resolve | unused binding |
| W0008 | warning | lint-post-resolve | documentation drift (declared; not emitted) |
| W0009 | warning | lint-post-resolve | deprecated API usage with the manifest-recorded replacement |
| W0010 | warning | lint-post-resolve | prefer use import for frequently qualified paths |
| W0011 | warning | lint-post-resolve | prefer else-if over nested else { if } |
| W0101 | warning | parse | deprecated import syntax; use `use` (declared; not emitted) |
| W0102 | warning | lint-post-resolve | component lowering note (declared; not emitted) |
<!-- END GENERATED:WARNING_CODE_CATALOGUE -->

---

### W0101 — deprecated `import <name>` syntax

| Field | Value |
|-------|-------|
| **Severity** | warning |
| **Phase** | parse |
| **Message** | deprecated import syntax |
| **Maturity** | declared; not currently emitted |

ADR-031 reserves this identity for the `import` to `use` migration. The current
parser does not yet emit the warning, so examples below describe the accepted
contract rather than observed compiler output.

```text
// Deprecated
import math

// Preferred
use math
```

Aliases migrate directly:

```text
import math as m  // W0101
use math as m
```

---

### W0102 — component lowering note

| Field | Value |
|-------|-------|
| **Severity** | warning |
| **Phase** | lint-post-resolve |
| **Maturity** | declared; not currently emitted |

Reserved for non-fatal Component Model lowering notes. This identity is
separate from W0101 so the accepted import-syntax migration contract remains
stable.

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
in `src/compiler/diagnostics/codes.ark` and `docs/data/warnings.toml`, update
this file to match.
`scripts/check/check-docs-consistency.py` includes a check for error-code-to-doc
alignment.

## See Also

- [diagnostics.md](diagnostics.md) — diagnostic system architecture
- [pipeline.md](pipeline.md) — compiler pipeline overview
- [../language/error-handling.md](../language/error-handling.md) — language-level error handling
