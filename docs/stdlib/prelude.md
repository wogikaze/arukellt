# Arukellt Standard Prelude

> **Status**: stable  
> **See also**: [ADR-036 (trait-based stdlib redesign)](../adr/ADR-036-trait-stdlib-redesign.md), [prelude-migration.md](prelude-migration.md)

The Arukellt compiler automatically imports a small set of symbols into every
module. This page describes the design of that prelude and the ongoing
transition to a trait-based standard library (ADR-036).

## Tiny Prelude (v3 canonical set)

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
`earliest_removal`, and the currently usable replacement—is
[`monomorphic-deprecation.md`](monomorphic-deprecation.md). Other monomorphic
helpers remain stable or provisional until the manifest changes; this page does
not broaden their lifecycle state.

The manifest's `deprecated_by` value is the **current replacement**. Trait-based
method syntax is the planned end state from ADR-036, but must not be presented
as the current replacement until the compiler supports that path.

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
// deprecated — monomorphic free functions
let v: Vec<i32> = Vec_new_i32()
let doubled: Vec<i32> = map_i32_i32(v, fn(x: i32) -> i32 { x * 2 })
let s: String = concat("a", "b")
let n: String = i32_to_string(42)
```

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" -->
```ark
// planned end state — not the current migration target (post-#688)
let v: Vec<i32> = Vec::new()              // #697 generic Vec
let doubled: Vec<i32> = v.map(fn(x) { x * 2 })  // #691 Iterator trait
let s: String = "a".concat("b")           // String method
let n: String = 42.to_string()            // #702 Display trait
```

### Current migration table

The current table is generated from the manifest in
[`monomorphic-deprecation.md`](monomorphic-deprecation.md). Planned trait
replacements remain design information in ADR-036 and
[`trait-stdlib-redesign.md`](trait-stdlib-redesign.md).

### Thin wrapper policy (ADR-036 D5)

Per ADR-036 D5, free functions like `clone`, `eq`, and `i32_to_string` will
remain in the prelude as **thin wrappers** delegating to their trait impls.
This means existing call sites (`clone(s)`, `eq(a, b)`) continue to work
during the transition, while new code should prefer method syntax
(`s.clone()`, `a.eq(b)`).

## Why a tiny prelude?

* **Namespace hygiene**: a large prelude introduces many short names that can
  conflict with user-defined functions.
* **Discoverability**: explicit `use` imports make dependencies visible.
* **Tooling**: an IDE can show exactly which module a function comes from.

## Timeline

1. **stdlib design epoch 3 (current)**: selected manifest entries are
   deprecated; trait definitions exist in `std/core/*.ark` but trait method
   dispatch is not yet implemented (#688).
2. **Post-#688–#697**: trait dispatch implemented; bold cutover removes
   monomorphic APIs; prelude free functions become thin wrappers to trait impls.

## See Also

* [ADR-036: Trait-based Stdlib Redesign](../adr/ADR-036-trait-stdlib-redesign.md) — design decisions
* [prelude-migration.md](prelude-migration.md) — detailed migration reference
* [std/prelude.ark](../../std/prelude.ark) — implementation source
