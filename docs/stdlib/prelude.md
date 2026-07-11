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

## Legacy Compat (deprecated — bold cutover per ADR-036)

All **monomorphic Vec constructors** and **monomorphic higher-order functions**
in `std/prelude.ark` are marked `stability = "deprecated"` in `std/manifest.toml`
with `deprecated_by` pointing to the trait-based replacement. Per
[ADR-036](../adr/ADR-036-trait-stdlib-redesign.md) D2, **already-deprecated**
surfaces are eligible for bold cutover (direct removal without a further
deprecation window) once issues #688–#697 (trait-based stdlib redesign)
are complete. This is consistent with ADR-036's rule that `stable` APIs
require at least one release of deprecation — these APIs are already
past that stage.

### Correct migration target: trait-based method syntax

The correct replacement is **trait-based method syntax**, not the intermediate
`std::text` / `std::seq` explicit-import style that was previously recommended.

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
// preferred — trait-based method syntax (post-#688)
let v: Vec<i32> = Vec::new()              // #697 generic Vec
let doubled: Vec<i32> = v.map(fn(x) { x * 2 })  // #691 Iterator trait
let s: String = "a".concat("b")           // String method
let n: String = 42.to_string()            // #702 Display trait
```

### Migration table

| Deprecated (prelude) | Trait-based replacement | Issue |
|-----------------------|-------------------------|-------|
| `Vec_new_i32()` / `Vec_new_i64()` / … | `Vec::new()` (generic) | #697 |
| `map_i32_i32(v, f)` / `map_f64_f64(v, f)` / … | `v.map(f)` (Iterator trait) | #691 |
| `filter_i32(v, f)` / `filter_f64(v, f)` / … | `v.filter(f)` (Iterator trait) | #691 |
| `fold_i32_i32(v, init, f)` / … | `v.fold(init, f)` (Iterator trait) | #691 |
| `sort_i32(v)` / `sort_String(v)` / … | `v.sort()` (Vec method) | #697 |
| `concat(a, b)` | `a.concat(b)` (String method) | — |
| `split(s, delim)` | `s.split(delim)` (String method) | — |
| `join(v, sep)` | `v.join(sep)` (Iterator method) | #691 |
| `i32_to_string(n)` / `bool_to_string(b)` / … | `n.to_string()` (Display trait) | #702 |
| `clone(s)` | `s.clone()` (Clone trait) | #692 |
| `eq(a, b)` | `a.eq(b)` (Eq trait) | #695 |

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

1. **v3 (current)**: monomorphic APIs are deprecated; trait definitions exist
   in `std/core/*.ark` but trait method dispatch is not yet implemented (#688).
2. **Post-#688–#697**: trait dispatch implemented; bold cutover removes
   monomorphic APIs; prelude free functions become thin wrappers to trait impls.

## See Also

* [ADR-036: Trait-based Stdlib Redesign](../adr/ADR-036-trait-stdlib-redesign.md) — design decisions
* [prelude-migration.md](prelude-migration.md) — detailed migration reference
* [std/prelude.ark](../../std/prelude.ark) — implementation source
