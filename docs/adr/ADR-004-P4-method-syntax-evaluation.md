# ADR-004 P4: Method Syntax Evaluation

**Status**: DEFERRED
**Date**: 2025-07-22
**Relates to**: ADR-004 (trait strategy), Issue #157

---

## Context

ADR-004 deferred traits from v0 and established a phased introduction plan:
P1 (limited `for`), P2 (string interpolation), P3 (traits), **P4 (method syntax)**, P5 (operator overloading).

This document evaluates whether to introduce method syntax (e.g. `v.push(x)`)
and records the decision.

---

## 1. Current State: Function-Centric Design

Arukellt's stdlib uses free functions with the "subject" as the first argument:

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let v: Vec<i32> = Vec_new_i32()
push(v, 42)
let n: i32 = len(v)
let s: String = concat(a, b)
let lower: String = to_lower(s)
let parts: Vec<String> = split(s, delim)
```

Higher-order operations follow the same pattern:

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
let doubled: Vec<i32> = map_i32_i32(v, |x| x * 2)
let evens: Vec<i32> = filter_i32(v, |x| x % 2 == 0)
let total: i32 = fold_i32_i32(v, 0, |acc, x| acc + x)
```

There are no `impl` blocks in user code today. The spec reserves `trait` and
`impl` as v1 keywords and defines method call syntax (`expr.method(args)`) but
the compiler's stdlib surface is entirely function-based.

---

## 2. Pros of Current Approach

| Benefit | Detail |
|---------|--------|
| **Simpler parser** | No `.ident(args)` expression; call expressions are uniform `ident(args)`. |
| **No vtable overhead** | All dispatch is static; no trait-object indirection. |
| **Clear ownership** | The first argument is visibly passed; no hidden `self` semantics. |
| **LLM-friendly** | Resolution rules are trivial — name lookup in scope, no impl search. |
| **Flat namespace** | No ambiguity between field access and method call. |
| **Easier error messages** | "function `push` not found" vs "no method `push` for type `T`". |

---

## 3. Cons of Current Approach

| Drawback | Detail |
|----------|--------|
| **Verbose chaining** | `join(map_String_String(split(s, ","), trim), ";")` instead of `s.split(",").map(trim).join(";")`. |
| **Unfamiliar to most programmers** | Developers from Rust/Python/JS/Go expect `v.push(x)`. |
| **Type-suffixed names** | Without method resolution, HOFs require monomorphized names (`map_i32_i32`, `filter_String`). |
| **Discoverability** | IDEs cannot offer `.`-completion on a value to list applicable operations. |

---

## 4. Minimal Method Syntax Proposal — UFCS

**Uniform Function Call Syntax** (UFCS): `v.push(x)` desugars to `push(v, x)`.

### 4.1 Semantics

```
expr.name(args…)  ≡  name(expr, args…)
```

- The receiver `expr` becomes the first positional argument.
- Resolution: look up `name` as a free function whose first parameter type
  matches the type of `expr`. No `impl` block required.
- If both a field and a function match, field access takes priority (consistent
  with struct semantics).

### 4.2 Chaining Example

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
// Before (function-centric)
let result = join(map_String_String(split(s, ","), trim), ";")

// After (UFCS sugar)
let result = s.split(",").map_String_String(trim).join(";")
```

### 4.3 Interaction with Traits (P3)

If traits are later introduced, UFCS and trait methods coexist:

1. Trait methods (via `impl`) are resolved first.
2. If no trait method matches, fall back to UFCS free-function lookup.

This ordering avoids breaking existing code when traits are added.

---

## 5. Impact Analysis

| Compiler Phase | Change Required | Complexity |
|---------------|----------------|------------|
| **Parser** | Add `.ident(args)` as a postfix expression. Parse as `MethodCall(receiver, name, args)`. | Low — one new expression variant. |
| **Resolver** | Desugar `MethodCall` → `Call(name, [receiver, …args])`. Look up `name` in scope; verify first param type matches receiver. | Medium — new lookup path, potential ambiguity with fields. |
| **Type checker** | Infer receiver type to drive function lookup. If overloaded by type-suffix (`push` for `Vec<i32>` vs `Vec<String>`), resolver must select the correct monomorphized variant. | Medium — requires type-directed name resolution, which the current checker does not do. |
| **HIR / CoreHIR** | No structural change — desugared before lowering. | None. |
| **Emitter (Wasm)** | No change — sees only `Call` nodes. | None. |
| **Stdlib surface** | No change required. Existing `push`, `len`, `concat`, etc. work as-is. Optionally, HOF names could drop type suffixes if type-directed lookup is available. | None (immediate) / Medium (cleanup). |
| **Docs / Migration** | Document UFCS rules. Update examples. | Low. |

---

## 6. Recommendation

**Defer to post-v5.**

Rationale:

1. **Self-hosting first.** The v5 milestone targets self-hosting the compiler
   in Arukellt. Introducing method syntax before self-hosting is stable adds
   parser/resolver complexity that slows that goal.

2. **Current API is consistent.** The function-centric stdlib works. All
   operations are callable and composable. The verbosity cost is real but
   manageable for a self-hosting compiler.

3. **Backward-compatible sugar.** UFCS is pure syntactic sugar. It can be added
   at any time without breaking existing code or changing semantics.

4. **Type-directed resolution is a prerequisite.** The current resolver does
   name-only lookup. UFCS with monomorphized function names (`push` resolving
   to the right type variant) requires type-directed resolution, which is a
   non-trivial change best tackled after the type system is battle-tested by
   self-hosting.

5. **Traits (P3) should land first.** Method syntax is most valuable when
   combined with trait-based dispatch. Introducing UFCS before traits means
   two separate method-resolution systems; introducing them together is
   cleaner.

---

## 7. Decision

**DEFERRED**

### Start Condition

- v5 self-hosting is complete.
- Stdlib API surface is stable (no major renames or signature changes planned).
- P3 (traits) has landed or a decision has been made on trait timing.

### Exit Condition

When this evaluation is revisited, the outcome must be one of:

- **Adopt UFCS**: implement the minimal desugaring described in §4.
- **Adopt full method syntax**: require `impl` blocks (depends on P3).
- **Reject**: document why function-centric API is sufficient long-term.

### References

- `docs/adr/ADR-004-trait-strategy.md` — original trait deferral
- `docs/language/spec.md` §2.8, §3.6 — trait/method syntax spec
- `std/prelude.ark` — current function-centric stdlib
- `issues/open/157-adr004-method-syntax-evaluation.md` — tracking issue
