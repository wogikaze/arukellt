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
| `std::fs` | File read/write |
| `std::io` | Standard input/output streams |
| `std::time` | Monotonic clock and duration |
| `std::random` | PRNG and WASI random |
| `std::process` | Process exit/abort |
| `std::test` | Assertions and test utilities |
| `std::env` | Environment access |
| `std::cli` | CLI argument parsing |

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

## Deprecation Process

A Stable API is deprecated before removal:

1. Mark with `@deprecated` annotation and document the replacement.
2. Keep the deprecated API available for at least one major version.
3. Remove in the next major version with a migration guide.
