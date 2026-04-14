# Arukellt Standard Prelude

> **Status**: stable  
> **See also**: [migration/v2-to-v3.md](../migration/v2-to-v3.md), [prelude-migration.md](prelude-migration.md)

The Arukellt compiler automatically imports a small set of symbols into every
module. This page describes the design of that prelude and explains what changed
in v3.

## Tiny Prelude (v3 canonical set)

The following symbols are available without any explicit `use` statement.
They form the *tiny prelude* — the permanent auto-imported surface.

### Compiler-builtin types and constructors

These are handled by `ark-typecheck` and are always in scope.

| Symbol | Kind |
|--------|------|
| `Option<T>` | type |
| `Result<T, E>` | type |
| `String` | type |
| `Vec<T>` | type |
| `Some(v)` | constructor |
| `None` | constructor |
| `Ok(v)` | constructor |
| `Err(e)` | constructor |

### Functions provided by `std/prelude.ark`

| Function | Purpose |
|----------|---------|
| `panic(s: String)` | Abort with a message |
| `assert(cond: bool)` | Assert a condition |
| `String_from(s)` | Construct a String |
| `String_new()` | Construct an empty String |
| `i32_to_string(n)` | Format i32 as String |
| `bool_to_string(b)` | Format bool as String |

Plus scalar conversion intrinsics (`i32_to_i64`, `f64_to_f32`, …) and basic
math helpers (`abs`, `min`, `max`).

## Legacy Compat (deprecated in v3)

All **monomorphic Vec constructors** and **monomorphic higher-order functions**
in `std/prelude.ark` are deprecated in v3.  They remain auto-imported for
backward compatibility and will be removed in v4.

<!-- skip-doc-check -->
```ark
// v2 style — deprecated
let v: Vec<i32> = Vec_new_i32()
let doubled: Vec<i32> = map_i32_i32(v, fn(x: i32) -> i32 { x * 2 })
let s: String = concat("a", "b")
```

<!-- skip-doc-check -->
```ark
// v3 style — preferred
use std::collections::vec
use std::text

let v: Vec<i32> = vec::new_i32()
let s: String = text::concat("a", "b")
```

The complete old → new mapping is documented in
[migration/v2-to-v3.md](../migration/v2-to-v3.md).

## Why a tiny prelude?

* **Namespace hygiene**: a large prelude introduces many short names that can
  conflict with user-defined functions.
* **Discoverability**: explicit `use` imports make dependencies visible.
* **Tooling**: an IDE can show exactly which module a function comes from.

The v3 migration is a *two-step* process:

1. **v3**: deprecated warning (not an error), explicit-import style encouraged.
2. **v4**: legacy compat names removed from the prelude.

## See Also

- [prelude-migration.md](prelude-migration.md) — detailed migration reference
- [migration/v2-to-v3.md](../migration/v2-to-v3.md) — full old → new API map
- [std/prelude.ark](../../std/prelude.ark) — implementation source
