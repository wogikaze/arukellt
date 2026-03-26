# v1 Non-Goals

This document lists design decisions and implementation constraints that are explicitly **out of scope** or **prohibited** during v1 development.

## Prohibited Actions

1. **Do not elevate T1 implementation details to T3 semantics**
   - The current T1 (linear memory + WASI Preview 1) is a deployment convenience, not the reference model.
   - Language semantics are defined by T3 (Wasm GC + WASI p2).
   - AtCoder-targeted code paths must not influence the type system or runtime model.

2. **Do not implement operator overloading before traits**
   - Operator overloading is trait-based (`impl Add for Point`).
   - P5 (operator overload) depends on P3 (trait system) being complete.
   - ✅ Already enforced: M6 implements operators via `impl` method dispatch.

3. **Do not introduce traits without coherence/orphan rules**
   - Every `impl Trait for Type` must follow coherence rules.
   - Orphan impls (implementing external traits for external types) are prohibited.
   - Ambiguous impl resolution is an error, not a warning.

4. **Do not add native-only features to T4**
   - The native backend (T4) reproduces Wasm semantics, nothing more.
   - No platform-specific APIs or native-only optimizations.

5. **Do not start WASI p3 implementation**
   - WASI p3 specification is not finalized.
   - Wait for spec stabilization before any implementation work.

6. **Do not import borrow checker or ownership into language spec**
   - Arukellt uses GC-based value semantics (deep copy on assignment).
   - No `&mut`, no lifetimes, no ownership annotations.
   - Memory management is the runtime's responsibility.

7. **Do not expose allocator internals to user code**
   - `mem.__alloc` and `mem.__free` are internal implementation details.
   - No user-visible allocation/deallocation API.

## Scope Boundaries

| Decision | v0 | v1 | Rationale |
|---|---|---|---|
| Trait system | ❌ | ✅ | Core v1 feature |
| Method syntax | ❌ | ✅ | Readability improvement |
| Operator overload | ❌ | ✅ (post-trait) | Requires trait infrastructure |
| Borrow checker | ❌ | ❌ | GC-based semantics |
| WASI p3 | ❌ | ❌ | Spec not finalized |
| Native-only features | ❌ | ❌ | Wasm semantics only |
| User-visible allocator | ❌ | ❌ | Internal detail |
