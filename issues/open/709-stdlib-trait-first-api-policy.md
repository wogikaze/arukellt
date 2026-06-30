---
Status: open
Created: 2026-07-01
Updated: 2026-07-01
ID: 709
Track: stdlib-api
Depends on: "691, 695, 697, 703"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#691 Iterator, #695 Ord, #697 Vec<T> operations, #703 monomorphic API cutover"
Blocks v{N}: none
Priority: 1
Source: Stdlib trait-first direction 2026-07-01
---

# 709 — Stdlib trait-first API policy and i32 helper containment

## Summary

The stdlib public surface is still too close to a collection of low-level
`i32` helper functions. This hurts reuse, makes docs look sparse, and teaches
LLM-generated code to select concrete helper APIs instead of composable
traits, modules, structs, and `impl` methods.

This issue tracks the policy that `*_i32` and similar monomorphic helpers are
allowed only as low-level implementation details or temporary compiler/runtime
bridges. User-facing stdlib APIs should be trait-first and type-first:

- reusable traits such as `Iterator`, `IntoIterator`, `FromIterator`, `Ord`,
  `Clone`, `Hash`, `Display`, `Debug`, `Read`, and `Write`
- coherent modules such as `std::iter`, `std::collections`, `std::core::ops`
- public structs/enums such as `Vec<T>`, `Deque<T>`, iterator adapters, result
  and error types
- `impl` methods and associated functions instead of ad-hoc free functions

## Current state

- `std::seq` exposes eager `Vec<i32>` helpers.
- `std::collections::vec` is much thinner than the operations exposed through
  prelude intrinsics.
- Several APIs are named by concrete representation rather than reusable
  behavior (`*_i32`, `cmp_i32`, `sort_i32`, `map_i32_i32`, etc.).
- Generated stdlib docs expose few structs/enums, which is a symptom of the
  API surface not being modeled as reusable types.

## Required work

- [ ] Define the stdlib public API policy: monomorphic helpers are private,
      unstable, or explicitly low-level unless an issue grants an exception.
- [ ] Add an inventory of all public `*_i32`, `*_i64`, `*_f64`, and
      representation-specific helpers in stdlib manifests and docs.
- [ ] Classify each helper as:
      - low-level allowed
      - temporary bridge blocked by compiler limitation
      - must be replaced by trait / struct / `impl` API
- [ ] Link each replacement to an owning issue such as #691, #695, #697,
      #702, or #703.
- [ ] Define a generated scorecard showing public functions vs structs/enums,
      trait-backed APIs, and remaining monomorphic helpers.
- [ ] Update stdlib docs to state that high-level user code should prefer
      trait / struct / `impl` APIs over concrete helper functions.

## Acceptance

- [ ] The stdlib has a documented trait-first API policy.
- [ ] All public monomorphic helpers have an explicit classification and owner.
- [ ] Public docs no longer present `i32` helpers as the primary high-level API.
- [ ] A generated or checkable scorecard tracks the remaining helper count.
- [ ] #703 has enough inventory data to delete or hide obsolete helpers.

## References

- #691 (`Iterator`, `IntoIterator`, `FromIterator`)
- #695 (`Ord` / `PartialOrd`)
- #697 (`Vec<T>` operation extension)
- #702 (`to_string` / `clone` / `hash` trait integration)
- #703 (monomorphic API bold cutover)
- `std/manifest.toml`
- `std/seq/mod.ark`
- `std/collections/`
