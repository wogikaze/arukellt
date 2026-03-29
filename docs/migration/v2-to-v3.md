# Migration Guide: v2 → v3 (Standard Library)

> Updated: 2026-03-29

## Overview

v3 organizes the ad-hoc monomorphic helper functions shipped through v2 (`Vec_new_i32`, `map_i32_i32`, etc.) into a structured standard library with a module system, stability labels, and generated reference documentation. Existing programs continue to compile, but the recommended import style changes from bare function names to `use std::*` paths.

## Key Changes

### 1. Module System (`use std::*`)

**v2:** All stdlib functions live in the global scope as monomorphic names.

```ark
let v = Vec_new_i32()
Vec_push_i32(v, 42)
let s = i64_to_string(n)
```

**v3:** Functions are organized into modules. The prelude re-exports the most common names, so most code continues to work without changes. For non-prelude functions, explicit `use` is required.

```ark
use std::collections::HashMap
use std::text::parse_i64

let v = Vec_new_i32()          // still available via prelude
let m = HashMap_new_i32_i32()  // requires `use std::collections::HashMap`
```

### 2. API Naming Convention

Stdlib modules follow a consistent naming scheme:

| Module | Contents |
|--------|----------|
| `std::core` | Fundamental types, conversions, numeric ops |
| `std::text` | String manipulation, parsing (`parse_i32`, `parse_i64`, `parse_f64`) |
| `std::bytes` | Byte-level operations |
| `std::collections` | `Vec`, `HashMap`, sequence utilities |
| `std::fs` | File system access (WASI-gated) |
| `std::io` | I/O primitives (`println`, `eprintln`, `read_line`) |
| `std::time` | Clock access (WASI-gated) |
| `std::random` | Random number generation (WASI-gated) |
| `std::process` | `exit`, process control |
| `std::env` | Environment variable access |
| `std::cli` | Argument parsing utilities |
| `std::wasm` | Wasm-specific intrinsics |
| `std::wit` | WIT interop helpers |
| `std::component` | Component Model helpers |

### 3. Scalar Type Completeness

v3 completes the scalar type surface. All scalar types now have consistent conversion, comparison, and arithmetic functions across the stdlib.

### 4. Stability Labels

Every public stdlib function is annotated with a stability label:

| Label | Meaning |
|-------|---------|
| **Stable** | API frozen; no breaking changes without a major version bump |
| **Unstable** | API may change in future versions |
| **Deprecated** | Scheduled for removal; migration guidance provided |

See `docs/stdlib/stability-policy.md` for the full policy.

### 5. Prelude Migration

The prelude (`std::prelude`) automatically imports the most frequently used names. Functions removed from the prelude emit a deprecation warning with guidance on the new import path.

See `docs/stdlib/prelude-migration.md` for the complete mapping.

### 6. Generated Reference Documentation

Stdlib reference docs are now generated from a manifest. Regenerate with:

```bash
# regenerate stdlib reference after changes
arukellt doc --stdlib
```

The generated output lives at `docs/stdlib/reference.md` and `docs/stdlib/modules/`.

## Unchanged Behavior

- T1 (`wasm32-wasi-p1`) and T3 (`wasm32-wasi-p2`) compilation paths are unaffected.
- Component Model features from v2 (`--emit component`, `--emit wit`) remain available.
- All v2 programs compile without changes thanks to the prelude re-exports.

## Migration Checklist

- [ ] Review `docs/stdlib/prelude-migration.md` for any names removed from the prelude
- [ ] Add explicit `use std::*` imports for non-prelude stdlib functions
- [ ] Replace deprecated function names flagged by compiler warnings
- [ ] (Optional) Adopt `docs/stdlib/stability-policy.md` labels when writing library code
- [ ] (Optional) Regenerate stdlib reference if contributing to the stdlib

## Related Documents

- `docs/stdlib/reference.md` — generated stdlib reference
- `docs/stdlib/stability-policy.md` — stability label policy
- `docs/stdlib/prelude-migration.md` — prelude migration mapping
- `docs/process/roadmap-v3.md` — historical v3 roadmap
- `docs/current-state.md` — current project state
