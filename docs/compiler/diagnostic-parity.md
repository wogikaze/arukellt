# Diagnostic Parity: Rust Compiler vs Selfhost Compiler

> Comparison of error diagnostic output between the Rust-hosted compiler
> and the self-hosted compiler for representative error cases.
>
> **Issue:** #289  
> **Prerequisite:** #287 (selfhost-fixture-parity) — done

## Comparison Contract

For diagnostic parity, each error case must satisfy **all three** of:

| Field | Parity requirement |
|-------|-------------------|
| **Error code** | Identical (e.g. `E0100`, `E0200`, `W0007`) |
| **Line number** | Same primary span line (column may differ) |
| **Severity** | Same level: `error` vs `warning` vs `help` |

Message text wording does **not** need to match exactly. The selfhost
compiler may phrase errors differently as long as the structured fields
above agree.

### How diagnostics are formatted

**Rust compiler** (`crates/ark-diagnostics/`):

```
error[E0100|resolve]: unresolved name
  --> file.ark:2:13
   |
  2 |     let x = unknown_function()
   |             ^^^^^^^^^^^^^^^^ unresolved name `unknown_function`
```

**Selfhost compiler** (`src/compiler/main.ark`):

```
file.ark: error: 1 resolve error(s)
file.ark: error: undefined name: unknown_function
```

The selfhost compiler currently emits **flat text messages** with:
- File path prefix
- `error:` severity tag (no `warning:` level)
- Phase error count (e.g. `1 resolve error(s)`)
- Individual error messages (no error codes, no line numbers, no spans)

## Test Method

1. Used existing `.ark` fixtures from `tests/fixtures/diagnostics/`
2. Ran Rust compiler: `mise x -- ./target/debug/arukellt run <file> 2>&1`
3. Ran selfhost compiler: `wasmtime run --dir=. src/compiler/arukellt-s1.wasm -- compile <file> 2>&1`
4. Compared output for error code, line number, and severity

## Case-by-Case Comparison

### Case 1: Undefined Variable — `unresolved_name.ark`

**Source:**

```ark
fn main() {
    let x = unknown_function()
}
```

**Expected:** `error[E0100|resolve]: unresolved name`

| Field | Rust compiler | Selfhost compiler | Match? |
|-------|--------------|-------------------|--------|
| Error code | `E0100` | *(none)* | ❌ |
| Line number | line 2, col 13 | *(none)* | ❌ |
| Severity | `error` | `error` | ✅ |
| Detected? | ✅ Yes | ✅ Yes | ✅ |

**Rust output:**

```
warning[W0007|typecheck]: unused binding `x`
  --> tests/fixtures/diagnostics/unresolved_name.ark:2:5

error[E0100|resolve]: unresolved name
  --> tests/fixtures/diagnostics/unresolved_name.ark:2:13
   |
  2 |     let x = unknown_function()
   |             ^^^^^^^^^^^^^^^^ unresolved name `unknown_function`
```

**Selfhost output:**

```
tests/fixtures/diagnostics/unresolved_name.ark: error: 1 resolve error(s)
tests/fixtures/diagnostics/unresolved_name.ark: error: undefined name: unknown_function
```

**Notes:** Selfhost correctly detects the undefined name and reports it at the
resolve phase. However, it lacks error code (`E0100`), line/column span, and
the `[code|phase]` bracketed format. The Rust compiler also emits a `W0007`
warning for the unused binding — selfhost does not emit warnings at all.

---

### Case 2: Type Mismatch — `type_mismatch.ark`

**Source:**

```ark
fn main() {
    let x: i32 = "hello"
}
```

**Expected:** `error[E0200|typecheck]:`

| Field | Rust compiler | Selfhost compiler | Match? |
|-------|--------------|-------------------|--------|
| Error code | `E0200` | *(none)* | ❌ |
| Line number | *(no span on primary)* | *(none)* | — |
| Severity | `error` | *(no error emitted)* | ❌ |
| Detected? | ✅ Yes | ❌ **No** | ❌ |

**Rust output:**

```
warning[W0007|typecheck]: unused binding `x`
  --> tests/fixtures/diagnostics/type_mismatch.ark:2:5

error[E0200|typecheck]: expected `i32`, found `String`
   = expected: i32
   = actual: String
```

**Selfhost output:**

```
compilation succeeded (phase 6)
ok: 417 bytes
```

**Notes:** The selfhost typechecker does **not** detect the type mismatch.
It compiles the file successfully through all 6 phases. This indicates the
selfhost typechecker does not yet enforce type compatibility on let-bindings
with explicit type annotations.

---

### Case 3: Syntax Error (Unexpected Token) — `unexpected_token.ark`

**Source:**

```ark
fn main() {
    let x = ;
}
```

**Expected:** `error[E0001|parse]:`

| Field | Rust compiler | Selfhost compiler | Match? |
|-------|--------------|-------------------|--------|
| Error code | `E0001` | *(none)* | ❌ |
| Line number | line 2, col 13 | *(none)* | ❌ |
| Severity | `error` | `error` | ✅ |
| Detected? | ✅ Yes | ✅ Yes | ✅ |

**Rust output:**

```
error[E0001|parse]: expected expression, found `Semi`
  --> tests/fixtures/diagnostics/unexpected_token.ark:2:13
   |
  2 |     let x = ;
   |             ^ here
```

**Selfhost output:**

```
tests/fixtures/diagnostics/unexpected_token.ark: error: 2 parse error(s)
tests/fixtures/diagnostics/unexpected_token.ark: error: unexpected token in expression: 73
tests/fixtures/diagnostics/unexpected_token.ark: error: expected token kind 73, got 0
```

**Notes:** Selfhost correctly rejects the syntax error at the parse phase.
However, it reports raw token kind numbers (73 = `Semi`, 0 = `Eof`) instead of
human-readable token names. No error code, no line number.

---

### Case 4: Missing Brace — `missing_brace.ark`

**Source:**

```ark
fn main() {
    let x = 42
```

**Expected:** `error[E0002|parse]:`

| Field | Rust compiler | Selfhost compiler | Match? |
|-------|--------------|-------------------|--------|
| Error code | `E0002` | *(none)* | ❌ |
| Line number | line 4 (EOF) | *(none)* | ❌ |
| Severity | `error` | `error` | ✅ |
| Detected? | ✅ Yes | ✅ Yes | ✅ |

**Rust output:**

```
error[E0002|parse]: expected `RBrace`, found `Eof`
   = expected: RBrace
   = actual: Eof
  --> tests/fixtures/diagnostics/missing_brace.ark:4:1
   |
  4 |
   | ^ here
```

**Selfhost output:**

```
tests/fixtures/diagnostics/missing_brace.ark: error: 1 parse error(s)
tests/fixtures/diagnostics/missing_brace.ark: error: expected token kind 73, got 0
```

**Notes:** Both compilers detect the missing brace. Selfhost reports raw token
kind numbers instead of token names. Token kind 73 corresponds to `RBrace`
(closing brace) and 0 to `Eof`.

---

### Case 5: Duplicate Definition — `duplicate_def.ark`

**Source:**

```ark
fn foo() -> i32 { 1 }
fn foo() -> i32 { 2 }
fn main() { foo() }
```

**Expected:** `error[E0101|resolve]: duplicate definition`

| Field | Rust compiler | Selfhost compiler | Match? |
|-------|--------------|-------------------|--------|
| Error code | `E0101` | *(none)* | ❌ |
| Line number | line 5 | *(none)* | ❌ |
| Severity | `error` | `error` | ✅ |
| Detected? | ✅ Yes | ✅ Yes | ✅ |

**Rust output:**

```
error[E0101|resolve]: duplicate definition
  --> tests/fixtures/diagnostics/duplicate_def.ark:5:1
   |
  5 | fn foo() -> i32 {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^ duplicate definition of `foo`
```

**Selfhost output:**

```
tests/fixtures/diagnostics/duplicate_def.ark: error: 1 resolve error(s)
tests/fixtures/diagnostics/duplicate_def.ark: error: duplicate definition: foo
```

**Notes:** Both compilers detect and report the duplicate definition at the
resolve phase. Messages are semantically equivalent. Missing: error code and
line number on the selfhost side.

---

### Case 6: Wrong Argument Count — `wrong_arg_count.ark`

**Source:**

```ark
fn add(a: i32, b: i32) -> i32 { a + b }
fn main() { add(1, 2, 3) }
```

**Expected:** `error[E0202|typecheck]:`

| Field | Rust compiler | Selfhost compiler | Match? |
|-------|--------------|-------------------|--------|
| Error code | `E0202` | *(none)* | ❌ |
| Line number | *(no span)* | *(none)* | — |
| Severity | `error` | *(no error emitted)* | ❌ |
| Detected? | ✅ Yes | ❌ **No** | ❌ |

**Rust output:**

```
error[E0202|typecheck]: expected 2 argument(s), found 3
```

**Selfhost output:**

```
compilation succeeded (phase 6)
ok: 461 bytes
```

**Notes:** The selfhost typechecker does not validate argument count. The file
compiles through all 6 phases with no errors.

---

### Case 7: Immutable Mutation — `immutable_mutation.ark`

**Source:**

```ark
fn main() {
    let x: i32 = 10
    x = 20
}
```

**Expected:** `error[E0207|typecheck]:`

| Field | Rust compiler | Selfhost compiler | Match? |
|-------|--------------|-------------------|--------|
| Error code | `E0207` | *(none)* | ❌ |
| Line number | line 3 | *(none)* | ❌ |
| Severity | `error` | *(no error emitted)* | ❌ |
| Detected? | ✅ Yes | ❌ **No** | ❌ |

**Rust output:**

```
error[E0207|typecheck]: cannot assign to immutable variable `x`
  --> tests/fixtures/diagnostics/immutable_mutation.ark:3:5
   |
  3 |     x = 20
   |     ^ cannot assign to immutable variable
```

**Selfhost output:**

```
compilation succeeded (phase 6)
ok: 413 bytes
```

**Notes:** The selfhost compiler does not enforce immutability.

## Summary Table

| # | Case | Error Code | Rust detects? | Selfhost detects? | Code match | Line match | Severity match |
|---|------|-----------|---------------|-------------------|------------|------------|----------------|
| 1 | Undefined variable | E0100 | ✅ | ✅ | ❌ | ❌ | ✅ |
| 2 | Type mismatch | E0200 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 3 | Unexpected token | E0001 | ✅ | ✅ | ❌ | ❌ | ✅ |
| 4 | Missing brace | E0002 | ✅ | ✅ | ❌ | ❌ | ✅ |
| 5 | Duplicate definition | E0101 | ✅ | ✅ | ❌ | ❌ | ✅ |
| 6 | Wrong arg count | E0202 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 7 | Immutable mutation | E0207 | ✅ | ❌ | ❌ | ❌ | ❌ |

**Full parity check** (`scripts/check/check-selfhost-parity.sh --diag`):
23 diagnostic fixtures tested — 0 pass, 0 fail, **23 skip** (selfhost does
not emit the expected `error[Exxxx|phase]:` pattern for any fixture).

## Divergence List

### Structural divergences (affect all cases)

1. **No error codes:** Selfhost emits `error:` without `[E0xxx|phase]` brackets.
   The Rust format is `error[E0100|resolve]:` — selfhost is `error: undefined name: ...`.

2. **No line/column spans:** Selfhost error messages include the file path but
   no line number, column, or source snippet. The Rust compiler shows full
   `-->` span annotations with source context.

3. **No warning severity:** Selfhost only emits `error:` messages. The Rust
   compiler also emits `warning[W0xxx|phase]:` for unused bindings, unused
   imports, and other non-fatal diagnostics. Selfhost has no warning pathway.

4. **Raw token kind numbers:** Parser errors in the selfhost compiler report
   numeric token kind IDs (e.g. `73`, `0`) instead of human-readable names
   (`Semi`, `Eof`, `RBrace`).

### Detection divergences (specific error types not caught)

| Error class | Rust code | Selfhost status | Phase |
|-------------|-----------|----------------|-------|
| Type mismatch | E0200 | **Not detected** — compiles successfully | typecheck |
| Wrong argument count | E0202 | **Not detected** — compiles successfully | typecheck |
| Immutable mutation | E0207 | **Not detected** — compiles successfully | typecheck |
| Missing type annotation | E0201 | Detected as **parse error** (not typecheck) | parse (wrong phase) |
| Non-exhaustive match | E0204 | **Not detected** — compiles successfully | typecheck |
| Mismatched match arms | E0205 | **Not detected** — compiles successfully | typecheck |
| `?` type mismatch | E0210 | **Not detected** — compiles successfully | typecheck |
| Unused binding | W0007 | **Not emitted** (no warning support) | typecheck |
| Unused import | W0006 | **Not emitted** (no warning support) | resolve |

### What works

| Error class | Rust code | Selfhost status |
|-------------|-----------|----------------|
| Undefined variable / function | E0100 | ✅ Detected at resolve phase |
| Duplicate definition | E0101 | ✅ Detected at resolve phase |
| Unexpected token | E0001 | ✅ Detected at parse phase |
| Missing brace / token | E0002 | ✅ Detected at parse phase |

## Parity Gap Assessment

The selfhost compiler's diagnostic system is at an **early stage**:

- **Lex + parse errors:** Detected reliably, but reported without error codes
  or source locations.
- **Resolve errors:** Core cases (undefined name, duplicate def) are detected
  and reported. Missing error codes and locations.
- **Typecheck errors:** **Not implemented.** The selfhost typechecker does not
  reject type mismatches, wrong argument counts, immutability violations, or
  exhaustiveness failures. These programs compile to Wasm successfully (and
  may produce incorrect runtime behavior).
- **Warnings:** Not supported at all. No `W0xxx` diagnostic pathway exists.

### Priority order for closing the gap

1. **Error codes + line numbers** (structural) — add `[Exxxx|phase]` format
   and source span tracking to satisfy the parity contract.
2. **Typecheck error detection** — implement type mismatch (E0200), argument
   count (E0202), and immutability (E0207) checks.
3. **Token name rendering** — replace numeric kind IDs with human-readable
   token names in parse errors.
4. **Warning pathway** — add `warning:` severity level for W0xxx diagnostics.

## Verification

```
scripts/check/check-selfhost-parity.sh --diag
  → diag-parity: pass=0 fail=0 skip=23 total=23
  → Exit 0 (no regressions; skips are expected at this stage)
```

## See Also

- [Error Codes Reference](error-codes.md)
- [Diagnostic System Design](diagnostics.md)
- [Bootstrap Guide](bootstrap.md)
- [Compiler Pipeline](pipeline.md)
