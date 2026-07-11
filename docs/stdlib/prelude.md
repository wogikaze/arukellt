# Arukellt Standard Prelude

> **Status**: stable  
> **Current contract sources**: `std/manifest.toml`, [stability policy](stability-policy.md)
>
> **Proposed design**: [ADR-036](../adr/ADR-036-trait-stdlib-redesign.md) (PROPOSED; non-normative)

The Arukellt compiler automatically imports a small set of symbols into every
module. This page describes only the current prelude contract. Proposed
trait-based changes live outside this stable reference.

## Tiny Prelude (stdlib design epoch 3)

The following symbols are available without any explicit `use` statement.
They form the *tiny prelude* — the permanent auto-imported surface.

### Compiler-builtin types and constructors

These are handled by `src/compiler/typechecker.ark` and are always in scope.

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

## Legacy compatibility and migration

Only entries marked `stability = "deprecated"` in `std/manifest.toml` are
deprecated. The generated, symbol-level list—including `deprecated_since`,
`earliest_removal`, and the manifest-recorded replacement—is
[`monomorphic-deprecation.md`](monomorphic-deprecation.md). Other monomorphic
helpers remain stable or provisional until the manifest changes; this page does
not broaden their lifecycle state.

The manifest's `deprecated_by` value records the intended replacement, not
proof that it is currently callable.
Availability caveats and verified migration evidence belong in the generated
migration table; this page does not recommend proposed trait methods.

### Current migration table

The current table is generated from the manifest in
[`monomorphic-deprecation.md`](monomorphic-deprecation.md). Proposed trait
replacements remain non-normative design information in ADR-036 and
[`trait-stdlib-redesign.md`](trait-stdlib-redesign.md).

## Why a tiny prelude?

* **Namespace hygiene**: a large prelude introduces many short names that can
  conflict with user-defined functions.
* **Discoverability**: explicit `use` imports make dependencies visible.
* **Tooling**: an IDE can show exactly which module a function comes from.

## See Also

* [ADR-036: Trait-based Stdlib Redesign](../adr/ADR-036-trait-stdlib-redesign.md) — proposed, non-normative design
* [std/prelude.ark](../../std/prelude.ark) — implementation source
