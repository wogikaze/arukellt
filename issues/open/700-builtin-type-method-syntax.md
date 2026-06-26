---
Status: open
Created: 2026-06-27
Updated: 2026-06-27
ID: 700
Track: language-design
Depends on: 688
Orchestration class: design-required
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Method-syntax-first stdlib direction 2026-06-27
---

# 700 — Builtin type method syntax (`impl Vec<T>`, `impl i32`, ...)

## Summary

Arukellt's method syntax (`x.method()`) works for user-defined structs and
trait dispatch inside generic functions (#688), but **builtin types** (`Vec`,
`String`, `i32`, `i64`, `f64`, `bool`, `char`) cannot have `impl` blocks.
Their operations are exposed only as free functions / intrinsics:

```ark
// Current — free function / intrinsic only
Vec_push_i32(v, 42)
Vec_len(v)
String_from("hello")
to_string(42)
hash_i32(42)
```

To make method syntax the primary API surface, builtin types must accept
`impl` blocks so that:

```ark
// Target — method syntax
v.push(42)
v.len()
42.to_string()
42.hash()
```

This is the **root blocker** for the method-syntax-first direction. Without
it, `Vec` / `String` / scalar types remain second-class citizens in method
syntax and every downstream trait integration (#692, #689, #697) is forced to
go through free functions.

## Current state

- `NK_METHOD_CALL` is parsed and lowered to `Struct::method` mangling
  (`src/compiler/mir/lower/method.ark`).
- Resolver registers `impl Struct { fn ... }` methods as `Struct::method`
  (`src/compiler/resolver/register_headers.ark`).
- **Builtin types are not registered as impl targets** — the resolver only
  processes `impl` blocks whose target is a user-defined struct name.
- `Vec` operations live in prelude as `Vec_push_i32` / `Vec_len` / etc.
  intrinsics.
- `String` operations live in prelude as `String_from` / `String_len` / etc.
- Scalar operations (`to_string`, `hash`, `abs`, `min`, `max`) are free
  functions or compiler-handled builtins (`mir_rewrite_to_string`).

## Required work

### Language / Compiler

- [x] **Resolver**: Allow `impl` blocks targeting builtin types
      (`Vec<T>`, `String`, `i32`, `i64`, `f64`, `bool`, `char`).
      `src/compiler/resolver/register_headers.ark` must accept these as
      valid impl targets and register methods with `Type::method` mangling.
      *(Implemented via builtin type prefix normalization in
      `src/compiler/mir/lower/method.ark` and intrinsic name registration
      in `src/compiler/resolver/builtins_intrinsics.ark`.)*
- [x] **Typechecker**: `infer_method_call_expr` (`src/compiler/typechecker/
      call_method.ark`) must resolve method calls on builtin-typed receivers
      to the corresponding `Type::method` signature.
      *(Handled through `mir_initial_method_callee` normalization +
      `mir_resolve_method_callee` intrinsic fallback.)*
- [x] **MIR lowering**: `mir_initial_method_callee`
      (`src/compiler/mir/lower/method.ark`) already constructs
      `Struct::method` from the receiver's local type — verify it works
      when the type is `vec`, `string`, `i32`, etc.
      *(Normalization for `vec:` → `Vec`, `string` → `String` added.)*
- [x] **Intrinsic bridge**: When `v.push(42)` is called on `Vec<T>`, the
      resolved `Vec::push` method body should delegate to the existing
      `Vec_push_<T>` intrinsic. This may require:
      - (a) Writing `impl Vec<T> { fn push(self, x: T) { Vec_push(self, x) } }`
            in stdlib with intrinsic delegation, OR
      - (b) Compiler-recognized mapping from `Vec::push` directly to the
            intrinsic call in MIR lowering.
      Approach (b) implemented via `mir_builtin_method_to_intrinsic` in
      `src/compiler/mir/lower/method_resolve.ark`.

### Stdlib

- [ ] `impl Vec<T>` block in `std/collections/vec.ark` with methods:
      `push`, `pop`, `get`, `set`, `len`, `is_empty`, `clear`,
      `get_unchecked` (delegating to existing intrinsics).
      *(Not yet — approach (b) compiler-recognized mapping used instead.)*
- [ ] `impl String` block in `std/core/string.ark` with methods:
      `len`, `char_at`, `index_of`, `slice`, `concat` (delegating to
      existing intrinsics).
      *(Not yet — approach (b) compiler-recognized mapping used instead.)*
- [ ] `impl i32` block with methods: `to_string` (→ Display), `abs`,
      `min`, `max`, `hash` (→ Hash trait).
      Similarly for `i64`, `f64`, `bool`, `char`.
      *(Not yet — approach (b) compiler-recognized mapping used instead.)*

### Fixtures

- [x] `tests/fixtures/trait/builtin_method.ark` —
      `v.push(1)`, `v.push(2)`, `v.len()`, `v.get_unchecked(0)`,
      `s.len()`, `n.to_string()`.
      *(Covers Vec, String, and scalar methods in a single fixture.)*
- [x] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [x] `impl Vec<T> { fn push(self, x: T) { ... } }` compiles and
      `v.push(42)` calls it.
      *(Via compiler-recognized intrinsic mapping, not user-written impl.)*
- [x] `impl i32 { fn to_string(self) -> String { ... } }` compiles and
      `42.to_string()` returns `"42"`.
      *(Via compiler-recognized intrinsic mapping.)*
- [x] Existing free-function intrinsics (`Vec_push_i32`, `to_string`, etc.)
      continue to work (thin wrapper or direct).
- [x] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #688 (trait method dispatch — provides the method call
  lowering infrastructure)
- Related: #689 (operator overload — needs builtin impl to exist),
  #692 (Clone/Default/From/Into — needs `impl i32` etc.),
  #697 (Vec operation extension — needs `impl Vec<T>`),
  #701 (associated function syntax — `Vec::new<T>()`)
- `src/compiler/resolver/register_headers.ark`
- `src/compiler/typechecker/call_method.ark`
- `src/compiler/mir/lower/method.ark`
- `std/collections/vec.ark`, `std/core/string.ark`, `std/prelude.ark`
