# Standard Library Stability Policy

## Stability Labels

Every module and public API in the Arukellt standard library carries one of three stability labels.

### Stable

- Backward-compatible within a major version.
- Breaking changes only occur on major version bumps.
- Suitable for production use.

### Experimental

- API may change in **minor** versions without notice.
- Functionality is available but the interface is not yet finalized.
- Marked with ⚠️ in documentation.

### Internal

- Used by the compiler and runtime only.
- No public API guarantee — may change or be removed at any time.
- Not intended for user code.

---

## Current Module Classification

### Stable

| Module | Description |
|--------|-------------|
| `prelude` | Auto-imported core functions |
| `std::core` | Option, Result, math, type conversion, panic/assert |
| `std::text` | String manipulation |
| `std::bytes` | Binary data, ByteBuf, ByteCursor, LEB128, encoding |
| `std::collections` | Vec, HashMap, HashSet, Deque (hash/linear/ordered) |
| `std::seq` | Lazy sequence combinators |
| `std::path` | Path manipulation (string-based) |
| `std::time` | Pure duration arithmetic |
| `std::random` | Deterministic seeded helpers |
| `std::test` | Assertions and test utilities |
| `std::host::stdio` | Standard output/error access |
| `std::host::fs` | File read/write |
| `std::host::process` | Process exit/abort |
| `std::host::env` | Environment and CLI arguments |
| `std::host::clock` | Monotonic host clock |
| `std::host::random` | Host entropy |

### Experimental

| Module | Description |
|--------|-------------|
| `std::wasm` | Wasm binary types and builder |
| `std::wit` | WIT type constants and naming |
| `std::component` | Component Model ABI metadata |
| `std::json` | JSON primitive stringify/parse |
| `std::toml` | TOML line parser |
| `std::csv` | CSV line splitter |
| `std::collections::compiler` | Compiler-internal collection variants |

---

## Promotion Process

An Experimental module becomes Stable when:

1. The API has been unchanged for at least one minor release cycle.
2. Test coverage meets the project baseline.
3. At least one ADR documents the design rationale.
4. The change is recorded in the changelog.

## Error / Result Naming Conventions

The stdlib follows consistent naming patterns for functions that can fail:

### Return type conventions

| Pattern | Returns | Example |
|---------|---------|---------|
| `parse_*` | `Result<T, String>` | `parse_i32("42")` → `Ok(42)` |
| `try_*` | `Result<T, String>` | Reserved for fallible operations |
| `*_or` | `T` (with fallback) | `unwrap_or(result, default)` |

### Result / Option builtins (prelude)

| Function | Signature | Description |
|----------|-----------|-------------|
| `is_ok` | `(Result<T, E>) -> bool` | Test if Result is Ok |
| `is_err` | `(Result<T, E>) -> bool` | Test if Result is Err |
| `unwrap` | `(Result<T, E>) -> T` | Extract Ok or panic |
| `unwrap_or` | `(Result<T, E>, T) -> T` | Extract Ok or return default |
| `is_some` | `(Option<T>) -> bool` | Test if Option is Some |
| `is_none` | `(Option<T>) -> bool` | Test if Option is None |
| `unwrap` | `(Option<T>) -> T` | Extract Some or panic |
| `unwrap_or` | `(Option<T>, T) -> T` | Extract Some or return default |

### Host vs pure family errors

- **Host family** (`std::host::*`): Functions that interact with the OS return `Result` when the operation can fail due to external state (e.g., `fs::read_file` returns `Result<String, String>`).
- **Pure family** (`std::collections`, `std::math`, etc.): Functions either return `Option` for partial operations or panic on precondition violations (e.g., `get` vs `get_unchecked`).

### Naming rules

1. Constructor variants use PascalCase: `Ok(v)`, `Err(e)`, `Some(v)`, `None`
2. Predicate builtins use snake_case: `is_ok`, `is_err`, `is_some`, `is_none`
3. Extraction builtins use snake_case: `unwrap`, `unwrap_or`, `expect`
4. Match arms use PascalCase constructors: `match r { Ok(v) => ..., Err(e) => ... }`

## Deprecation Process

A Stable API is deprecated before removal:

1. Mark with `@deprecated` annotation and document the replacement.
2. Keep the deprecated API available for at least one major version.
3. Remove in the next major version with a migration guide.
