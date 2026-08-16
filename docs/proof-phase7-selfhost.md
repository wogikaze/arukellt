# Selfhost proof Phase 7 source surface

The selfhost proof path assigns proof reference identities from checked `TypeInfo`
structure, not from display names. The current source-facing read-only memory
surface is deliberately narrow:

- `Vec<i32>` has proof TypeId `101` and element TypeId `1`.
- `Vec<bool>` has proof TypeId `102` and element TypeId `2`.
- `Vec<i64>` has proof TypeId `104` and element TypeId `4`.
- `Vec<T>` identity is derived from the `TY_VEC` tag and `type_args[0]`.
- Indexing lowers to canonical TypedCoreHIR v3 `array_get`.
- `Vec::len` lowers to canonical TypedCoreHIR v3 `array_len`.
- The canonical memory model is `arukellt-readonly-heap-v1`.
- Display names are diagnostic only and do not participate in proof identity.

The bridge remains fail-closed for opaque references, unsupported element types,
mutation, allocation, memory-bearing aggregates, reference-dependent loops or
calls, and other memory operations that are not represented by the Phase 7
semantic validator. Unsupported source constructs must be rejected before SMT
is generated; they must not be approximated by names or guessed layouts.

CI exercises two independent compiler-facing checks from the current Stage 2
runtime: structural proof-type-registry emission and a `Vec<i32>` contract that
is emitted as raw TypedCoreHIR, upgraded to canonical v3, lowered through the
Phase 7 VerifiedCore/SMT adapters, and discharged by real Z3.
