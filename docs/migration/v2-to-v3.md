# Migration Guide: v2 → v3 (Standard Library)

> Updated: 2026-03-28
> **Current-first note**: this guide explains the v2→v3 stdlib module system transition. For the current support matrix and known limitations, also check [`../current-state.md`](../current-state.md).

## Overview

v3 introduces the organized standard library module system on top of the v2 Component Model base. Existing v2 code continues to compile; however, the module import surface changes from ad-hoc prelude-only access to a structured `use std::*` system.

Key additions in v3:
- `use std::text`, `use std::bytes`, `use std::collections`, `use std::host` import namespaces
- Stability labels (`stable`, `provisional`, `experimental`, `deprecated`) on all public stdlib APIs
- Manifest-backed reference docs generated from `std/manifest.toml`
- Prelude surface cleaned up: some previously prelude-accessible functions now require explicit `use`

## Changes

### Module system (`use std::*`)

v3 introduces importable stdlib modules. Functions that were previously only accessible through the prelude or as builtins can now be explicitly imported:

```arukellt
use std::text;
use std::bytes;
use std::collections;
```

### Stability labels

All public stdlib APIs now carry one of four stability labels:

| Label | Meaning |
|-------|---------|
| `stable` | Backward-compatible within a major version |
| `provisional` | Usable but may change in minor versions |
| `experimental` | May change without notice |
| `deprecated` | Superseded — see migration guidance |

### Prelude changes

Some functions moved from the implicit prelude into importable modules. See [`docs/stdlib/prelude-migration.md`](../stdlib/prelude-migration.md) for the complete list.

### Deprecated API removal warnings

APIs marked `deprecated` in v3 emit `W0003` diagnostics. These APIs will be removed in a future major version.

```text
warning[W0003]: `old_fn` is deprecated — use `new_fn` instead
```

## Migration Steps

1. **Replace deprecated API usages** — run `arukellt check` to surface all `W0003` warnings and update call sites.

2. **Add explicit imports** for any stdlib functions that moved out of the prelude. See `docs/stdlib/prelude-migration.md` for the mapping.

3. **Update toolchain** — `mise install` to get the v3 compiler.

4. **Verify** — `arukellt check` should pass with no errors and only expected warnings.

## Unchanged Behavior

- All v2 CLI flags (`--emit component`, `--emit wit`, `--emit all`, `--target wasm32-wasi-p2`) continue to work.
- Core language syntax (functions, structs, enums, generics) is unchanged.
- T1 (`wasm32-wasi-p1`) compatibility path remains available.
- Existing Component Model binaries built with v2 remain valid.

## Known Limitations

- `std::collections::HashMap` is a GC-native rehashing map; linear-memory map is not available.
- `std::host` I/O functions depend on WASI P2 capabilities; some are not available in all runtimes.
- Nested generics restrictions from v2 are still in effect in v3.

## See Also

- [`docs/stdlib/reference.md`](../stdlib/reference.md) — generated stdlib API reference
- [`docs/stdlib/stability-policy.md`](../stdlib/stability-policy.md) — stability label policy
- [`docs/stdlib/prelude-migration.md`](../stdlib/prelude-migration.md) — prelude migration guide
- [`docs/current-state.md`](../current-state.md) — current support matrix
- [`v1-to-v2.md`](v1-to-v2.md) — previous migration guide
