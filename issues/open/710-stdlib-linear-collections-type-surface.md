---
Status: open
Created: 2026-07-01
Updated: 2026-07-01
ID: 710
Track: stdlib-api
Depends on: "691, 697, 701, 707, 709"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#691 Iterator, #697 Vec<T>, #701 associated functions, #707 Self return type, #709 trait-first policy"
Blocks v{N}: none
Priority: 1
Source: Stdlib collection type-surface audit 2026-07-01
---

# 710 — Linear collection ADTs: `Deque<T>` / queue / stack / list type surface

## Summary

`std::collections::linear` is currently a weak abstraction boundary. APIs such
as deque / queue / stack / list should not primarily appear as miscellaneous
functions under a broad "linear" bucket. They should be modeled as public
collection types with coherent modules, constructors, methods, trait impls,
and generated docs entries under **Structs** / **Enums** where appropriate.

The desired shape is closer to:

- `std::collections::Deque<T>` or `std::collections::deque::Deque<T>`
- `std::collections::Queue<T>`
- `std::collections::Stack<T>`
- `std::collections::LinkedList<T>` if the implementation is meaningful
- iterator structs for borrowed, mutable, and owning traversal

The exact naming does not need to copy Rust blindly, but the public surface
should be type-first and reusable.

## Current state

- Generated docs expose too few collection structs/enums.
- Some collection-like APIs are grouped by a vague category rather than by
  concrete reusable ADTs.
- `Deque`-like functionality is not clearly visible as a struct with methods
  and trait implementations.
- `#697` tracks `Vec<T>` specifically, and `#687` tracks hash collections;
  neither fully owns linear collection ADTs.

## Required work

- [ ] Audit `std::collections::linear` and adjacent collection modules.
- [ ] Decide the public namespace:
      - `std::collections::Deque<T>` style
      - `std::collections::deque::Deque<T>` style
      - or another documented stdlib convention
- [ ] Define public struct surfaces for deque / queue / stack / list where
      the API is intended to exist.
- [ ] Add constructors via associated functions (`Deque::new`,
      `Deque::with_capacity`) once #701 allows the desired syntax.
- [ ] Add methods via `impl` blocks: `push_front`, `push_back`, `pop_front`,
      `pop_back`, `len`, `is_empty`, `clear`, and equivalent per type.
- [ ] Add trait impl plan: `IntoIterator`, `FromIterator`, `Clone`, `Debug`,
      `PartialEq`, `Eq`, `Hash` where applicable.
- [ ] Make generated docs list these items under **Structs** / **Enums**,
      not only under module/function lists.
- [ ] Mark old free-function or helper-style APIs as deprecated or internal
      once the type surface is in place.

## Acceptance

- [ ] Linear collection APIs have named public structs or explicitly documented
      reasons for being absent.
- [ ] `Deque<T>` or the chosen equivalent appears under **Structs** in std docs.
- [ ] The chosen namespace is documented and not just a generic "linear"
      bucket.
- [ ] Method and trait implementation plans are linked to #691, #697, #701,
      and #707.
- [ ] Old helper-style collection APIs have migration paths.

## References

- #687 (HashMap / HashSet parity)
- #691 (`Iterator` / `IntoIterator` / `FromIterator`)
- #697 (`Vec<T>` operation extension)
- #701 (associated function syntax)
- #707 (`Self` return type support)
- #709 (trait-first API policy)
- `std/collections/`
- `std/manifest.toml`
