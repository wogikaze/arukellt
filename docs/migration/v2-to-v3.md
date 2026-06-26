# Migration Guide: v2 → v3 (Standard Library)

> Updated: 2026-06-27
> **Current-first note**: this guide explains the v2→v3 stdlib module system transition. For the trait-based redesign plan (post-#688), see [ADR-036](../adr/ADR-036-trait-stdlib-redesign.md). For the current support matrix and known limitations, also check [`../current-state.md`](../current-state.md).

## Overview

v3 introduces the organized standard library module system on top of the v2 Component Model base. Existing v2 code continues to compile; however, the module import surface changes from ad-hoc prelude-only access to a structured `use std::*` system.

Key additions in v3:
- `use std::text`, `use std::bytes`, `use std::collections`, `use std::host` import namespaces
- Stability labels (`stable`, `provisional`, `experimental`, `deprecated`) on all public stdlib APIs
- Manifest-backed reference docs generated from `std/manifest.toml`
- Prelude surface cleaned up: some previously prelude-accessible functions now require explicit `use`
- Trait definitions in `std/core/*.ark` (`Display`, `Eq`, `Hash`, `Add`, `Sub`, …) — trait method dispatch pending #688

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

### Trait-based redesign (ADR-036, post-#688)

The v3 `std::text` / `std::seq` explicit-import style is an **intermediate
step**. The final migration target is **trait-based method syntax** as
specified in [ADR-036](../adr/ADR-036-trait-stdlib-redesign.md).

Once issues #688–#697 (trait method dispatch, Iterator trait, Clone/Display/Eq
traits, Vec method extension, etc.) are complete, a **bold cutover** (ADR-036
D2) will remove monomorphic APIs directly — without a long deprecation period.

| Deprecated (prelude) | ~~Intermediate (v3)~~ | **Final (post-#688, ADR-036)** |
|-----------------------|------------------------|----------------------------------|
| `Vec_new_i32()` | ~~`vec::new_i32()`~~ | `Vec::new()` (generic, #697) |
| `map_i32_i32(v, f)` | ~~`seq::map(...)`~~ | `v.map(f)` (Iterator, #691) |
| `sort_i32(v)` | ~~`seq::sort_i32(v)`~~ | `v.sort()` (Vec method, #697) |
| `concat(a, b)` | ~~`text::concat(a, b)`~~ | `a.concat(b)` (String method) |
| `i32_to_string(n)` | ~~`text::format_i32(n)`~~ | `n.to_string()` (Display, #702) |
| `clone(s)` | — | `s.clone()` (Clone, #692) |
| `eq(a, b)` | — | `a.eq(b)` (Eq, #695) |

Per ADR-036 D5, prelude free functions (`clone`, `eq`, `i32_to_string`, …)
will remain as thin wrappers delegating to trait impls, so existing call sites
continue to work during the transition.

## Migration Steps

1. **Replace deprecated API usages** — run `arukellt check` to surface all `W0003` warnings and update call sites.

2. **Add explicit imports** for any stdlib functions that moved out of the prelude. See `docs/stdlib/prelude-migration.md` for the mapping.

3. **Prefer trait-based method syntax** for new code where trait impls already
   exist (`std/core/*.ark`). Full trait method dispatch requires #688.

4. **Update toolchain** — `mise install` to get the v3 compiler.

5. **Verify** — `arukellt check` should pass with no errors and only expected warnings.

## Unchanged Behavior

- All v2 CLI flags (`--emit component`, `--emit wit`, `--emit all`, `--target wasm32-wasi-p2`) continue to work.
- Core language syntax (functions, structs, enums, generics) is unchanged.
- T1 (`wasm32-wasi-p1`) compatibility path remains available.
- Existing Component Model binaries built with v2 remain valid.

## Known Limitations

- `std::collections::HashMap` is a GC-native rehashing map; linear-memory map is not available.
- `std::host` I/O functions depend on WASI P2 capabilities; some are not available in all runtimes.
- Nested generics restrictions from v2 are still in effect in v3.
- Trait method dispatch inside generic functions is not yet implemented (#688).

## See Also

- [`docs/adr/ADR-036-trait-stdlib-redesign.md`](../adr/ADR-036-trait-stdlib-redesign.md) — trait-based stdlib redesign strategy
- [`docs/stdlib/reference.md`](../stdlib/reference.md) — generated stdlib API reference
- [`docs/stdlib/stability-policy.md`](../stdlib/stability-policy.md) — stability label policy
- [`docs/stdlib/prelude-migration.md`](../stdlib/prelude-migration.md) — prelude migration guide
- [`docs/current-state.md`](../current-state.md) — current support matrix
- [`v1-to-v2.md`](v1-to-v2.md) — previous migration guide
