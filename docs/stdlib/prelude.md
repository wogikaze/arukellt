# Arukellt Standard Prelude

> **Status**: stable  
> **Current contract sources**: `std/manifest.toml`, [stability policy](stability-policy.md)
>
> **v2 policy**: [ADR-072](../adr/ADR-072-language-v2-prelude-and-keyword-policy.md)

The v2 contract defines a small stable set of symbols. The resolver now
auto-imports only that stable set; every other `pub` prelude decl stays
available through an explicit `use std::prelude::{name}` selective import
(enforced by #878). This page distinguishes those contracts rather than
treating every legacy entry as a new stability promise.

## Tiny Prelude (stdlib design epoch 3)

The following symbols form the v2 stable Prelude. They are available without
an explicit `use` statement.

### Compiler-builtin types and constructors

These are handled by `src/compiler/typechecker.ark` and are always in scope.

| Symbol | Kind |
|--------|------|
| `Option<T>` | type |
| `Result<T, E>` | type |
| `String` | type |
| `Vec<T>` | type |
| `Std` | namespace type (`Std::panic`) |
| `Some(v)` | constructor |
| `None` | constructor |
| `Ok(v)` | constructor |
| `Err(e)` | constructor |

`true` and `false` are lexical boolean literals, not Prelude values. They do
not have a second manifest or resolver entry.

### Functions provided by `std/prelude.ark`

| Function | Purpose |
|----------|---------|
| `Std::panic(message: String)` | Abort with a message (associated on `Std`) |
| `assert(cond: bool)` | Assert a condition |
| `String_from(s)` | Construct a String |
| `String_new()` | Construct an empty String |
| `char_to_string(c)` | Encode a Unicode scalar as String |

The intrinsic compatibility group (`len`, `push`, `pop`, `get`, `set`, and
`get_unchecked`) remains explicit in the manifest policy for existing compiler
and stdlib code. Other `prelude = true` entries are still explicit-import only:
bare calls fail with E0100 and a migration hint, and the name resolves only
through `use std::prelude::{name}` or its canonical replacement module.

The 27 method-replaceable prelude frees (`trim`, `split`, `join`, `abs`,
`min`, `max`, `values`, …) were retired on 2026-10-29 under
[ADR-091](../adr/ADR-091-prelude-free-function-exception.md). Their bare calls
now fail with E0100 and the receiver-method hint; see the
[before/after table](migration-guidance.md#prelude-free-function-retirement-adr-091).
`assert`, `String_from`, `String_new`, and `char_to_string` remain as the exact
ADR-091 stable-free exception. `push_char` stays free because the mutating
`s.push_char(c)` receiver method does not propagate its grown array to the
caller on the current lowering. `parse_i32` / `parse_i64` / `parse_f64` also
stay free: moving compiler/std call sites to `TryFrom` made the rebuilt s2 trap
while compiling any input. Both are `associated-or-method` residuals, not part
of the exception.

The bare `concat`, `char_at`, `print`, and `println` bindings were removed
from the published surface in the #917 0.x eradication wave
([0.x exception](stability-policy.md#0x-eradication-exception-917)). Calling
them still fails with E0100 and the resolver hint that names the replacement
(`a.concat(b)`, `byte_at`, and `std::host::stdio`).

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
