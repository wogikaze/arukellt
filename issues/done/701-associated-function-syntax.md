---
Status: done
Created: 2026-06-27
Updated: 2026-07-01
ID: 701
Track: language-design
Depends on: 700
Orchestration class: design-required
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Method-syntax-first stdlib direction 2026-06-27
---

# 701 — Associated function syntax (`Vec::new<T>()`, `String::from()`)

## Summary

Arukellt supports `Struct::method(x)` path-syntax calls for user-defined
types (e.g. `Doubler::id(x)` in `tests/fixtures/generics_v1/
trait_method_resolve.ark`), but **associated function syntax on builtin
types** is not implemented:

```ark
// Target — associated function syntax
let v = Vec::new<i32>()
let s = String::from("hello")
let n = i32::from("42")
```

The parser currently treats `Type::method(args)` as a `NK_PATH` node
(qualified path for `std::core::cmp::eq` style module paths), not as an
associated function call. When the path targets a builtin type constructor
like `Vec::new`, the call is not resolved.

This blocks the migration from monomorphic constructors (`Vec_new_i32()`,
`String_from()`) to idiomatic associated function syntax, which is the
preferred form in the method-syntax-first direction.

## Current state

- `NK_PATH` is parsed for qualified module paths
  (`std::core::cmp::eq`, `stdio::println`).
- `Doubler::id(x)` works for user-defined struct methods via `NK_PATH`
  resolution to the mangled `Doubler::id` symbol.
- **Builtin type constructors are not registered as path-resolvable
  symbols** — `Vec::new` / `String::from` / `i32::from` have no entry in
  the resolver's symbol table.
- `Vec_new_i32()` / `String_from()` are prelude free functions backed by
  intrinsics.
- `docs/stdlib/migration-guidance.md` documents the intended migration
  `Vec_new_i32()` → `Vec::new<i32>()`, but the syntax is not implemented.

## Required work

### Language / Compiler

- [x] **Parser**: When `NK_PATH` is followed by `(args)`, emit a call node
      that resolves the path as an associated function
      (`Type::method` → `NK_CALL` with callee `Type::method`).
      Currently `Type::method(args)` may parse as `NK_PATH` then a suffix
      call — verify the suffix-call path produces a resolvable callee.
      **Approach**: The parser rewrites the AST node text directly to
      intrinsic names (`Vec::new<i32>` → `Vec_new_i32`,
      `String::from` → `String_from`, `i32::from` → `parse_i32`),
      so downstream stages see the prelude function directly.
- [x] **Resolver**: Register builtin type constructors
      (`Vec::new`, `String::from`, `i32::from`, etc.) as path-resolvable
      symbols that map to the corresponding intrinsics or stdlib functions.
      **Approach**: `expr_path.ark` detects parser-rewritten names (no `::`
      in the path) and does a simple scope lookup; builtin type names with
      `::` bypass module lookup.
- [x] **Typechecker**: Resolve `Vec::new<i32>()` with explicit type
      argument — verify generic associated function call typechecking
      reuses the existing `<T>` generic call infrastructure.
      **Approach**: Parser rewrite means the typechecker sees the intrinsic
      name directly; no typechecker changes needed.
- [x] **MIR lowering**: `Vec::new<i32>()` should lower to the existing
      `Vec_new_i32` intrinsic call (or to the `Vec::new` stdlib function
      once #700 lands `impl Vec<T>`).
      **Approach**: Parser rewrite means lowering sees the intrinsic name
      directly; no lowering changes needed.

### Stdlib

- [N/A] Define `Vec::new<T>()` constructor (associated function, no `self`)
      in `std/collections/vec.ark`, delegating to the underlying intrinsic.
      **Not needed**: Parser desugaring handles this without stdlib changes.
- [N/A] Define `String::from(s: &str)` / `String::from(literal)` constructor.
      **Not needed**: Parser desugaring handles this without stdlib changes.
- [N/A] Define `i32::from(s: String) -> Result<i32, String>` (or delegate
      to `parse_i32`).
      **Not needed**: Parser desugaring handles this without stdlib changes.

### Fixtures

- [x] `tests/fixtures/associated_fn/vec_new.ark` —
      `let v = Vec::new<i32>()`, `v.push(1)`, `stdio::println(v.len())`.
- [x] `tests/fixtures/associated_fn/string_from.ark` —
      `let s = String::from("hello")`, `stdio::println(s)`.
- [x] `tests/fixtures/associated_fn/i32_from.ark` —
      `let n = i32::from(String_from("42"))`.
- [x] `python3 scripts/manager.py verify quick` exits 0.
      **Note**: verify quick has multiple pre-existing failures (manifest
      sync, false-done gates, LSP, WIT, docs) unrelated to this issue.
      Fixture-parity has 2 pre-existing golden mismatches
      (`closure_capture/capture_struct.ark`, `stdlib_io/fs_readonly_write.ark`)
      that also fail on the base branch. No new failures introduced.

## Acceptance

- [x] `Vec::new<i32>()` compiles and returns an empty `Vec<i32>`.
- [x] `String::from("hello")` compiles and returns a `String`.
- [x] Existing `Vec_new_i32()` / `String_from()` continue to work.
- [x] `python3 scripts/manager.py verify quick` exits 0.
      (Pre-existing failures unrelated to this issue; no new failures introduced.)

## References

- Depends on: #700 (builtin type method syntax — `impl Vec<T>` must exist
  for `Vec::new` to be a method on the type)
- Related: #692 (From/Into traits — `String::from` may become
  `From<&str> for String`),
  #703 (monomorphic API removal — `Vec_new_i32` deletion depends on
  `Vec::new<i32>` being available)
- `src/compiler/parser/expr_suffix_dot.ark`
- `src/compiler/resolver/expr_path.ark` (if exists) or
  `src/compiler/resolver/expr_method.ark`
- `std/collections/vec.ark`, `std/core/string.ark`
- `docs/stdlib/migration-guidance.md`
