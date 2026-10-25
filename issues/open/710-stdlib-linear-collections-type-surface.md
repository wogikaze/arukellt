---
Status: open
Created: 2026-07-01
Updated: 2026-10-25
ID: 710
Track: stdlib-api
Depends on: "691, 697, 701, 707, 709"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#691 Iterator, #697 Vec<T>, #701 associated functions, #707 Self return type, #709 trait-first policy; ADR-086 (PROPOSED) implementation prerequisites"
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
- The design is now recorded in
  [ADR-086](../../docs/adr/ADR-086-linear-collection-adts.md) as **PROPOSED**.
  Implementation has not started; the ADR is not an accepted current contract.

## Design decisions (ADR-086, PROPOSED)

The design record is
[ADR-086: std::collections の線形コレクション ADT](../../docs/adr/ADR-086-linear-collection-adts.md).
The decisions this issue must implement are:

- **Namespace:** use defining modules, not a flat `std::collections` namespace.
  - `std::collections::deque::Deque<T>`
  - `std::collections::deque::Queue<T>`
  - `std::collections::deque::Stack<T>`
  - `std::collections::linked_list::LinkedList<T>`
- **Surfaces:** all four are structs with private fields. `Queue<T>` and
  `Stack<T>` are restricted newtypes over `Deque<T>`. No public collection
  enum is introduced; `Enums` docs entries are not part of this work.
- **Constructors:** associated functions only. `new` on every type, plus
  `with_capacity` on `Deque` / `Queue` / `Stack`. `Deque::new<i32>()` is the
  canonical spelling.
- **Methods:**
  - `Deque<T>`: `push_front`, `push_back`, `pop_front`, `pop_back`, `front`,
    `back`, `len`, `is_empty`, `clear`, `capacity`, `reserve`, `iter`,
    `iter_mut`.
  - `Queue<T>`: `push` (back), `pop` (front), `front`, `back`, `len`,
    `is_empty`, `clear`, `capacity`, `reserve`, `iter`.
  - `Stack<T>`: `push` (back), `pop` (back), `peek` (back), `len`,
    `is_empty`, `clear`, `capacity`, `reserve`, `iter`.
  - `LinkedList<T>`: `push_front`, `push_back`, `pop_front`, `pop_back`,
    `front`, `back`, `len`, `is_empty`, `clear`, `iter`, `iter_mut`.
  - Empty pops / peeks return `Option<T>`, not a panic or a `0` sentinel.
- **Iterators:** `DequeIter<T>` / `DequeIterMut<T>` / `DequeIntoIter<T>` and
  `LinkedListIter<T>` / `LinkedListIterMut<T>` / `LinkedListIntoIter<T>`.
  Queue and Stack reuse the Deque iterator structs; no `QueueIter` /
  `StackIter` aliases. `iter_mut` is a cursor with
  `set_current(value) -> Option<T>`, not a `&mut T` reference.
- **Traits:** `IntoIterator`; `FromIterator` for the collection's own
  borrowed/owning iterators and `VecIter<T>`; `Clone`, `Debug`, `Eq` under
  the corresponding element bounds. No `Default`, `Hash`, `Ord`, `Display`,
  `Copy`, and no universal `FromIterator<I>`.
- **Docs data:** extend `std/manifest.toml` `[[types]]` with `kind`
  (`struct` / `enum` / `trait`), `doc`, `methods`, and `trait_impls`. Generated
  Markdown must group **Public Types** into **Structs / Enums / Traits**;
  `arukellt doc --html` must use `kind`, not a hardcoded enum-name list.
  `deprecated_by` must resolve `Owner::member` so a free function can point at
  its method replacement.
- **Migration:** `deque_*` are stable and require an ADR-014 deprecation
  window with `deprecated_by = "Deque::<method>"`. `linked_list_*` are not
  manifest-published and can be removed with a migration note after in-tree
  callers move. `pq_*` (PriorityQueue) is out of scope and keeps its current
  owner.
- **Stability:** new types and iterators are `provisional`; promote to
  `stable` only after both-target parity, docs entries, and the planned trait
  impls land. Do not mark them stable merely because the old free functions
  were stable.

### Implementation prerequisites

- Generic associated constructors on declared generic structs
  (`Deque::new<T>()`) must typecheck and lower on both targets.
- Bounded parametric impls (`impl<T: Clone> Clone for Deque<T>`) must be
  usable; ADR-078 permits the shape, but this issue needs fixtures proving the
  parser / typechecker / lowering path.
- The `[[types]]` docs schema and `deprecated_by` resolution above must exist
  before old free functions can be deprecated against methods.
- `wasm32-gc` and `wasm32` parity fixtures are required for each type surface.

### Non-goals

- `PriorityQueue<T>` / `pq_*` conversion. It is a heap, not a linear ADT.
- `Vec<T>` / `HashMap<K, V>` surface changes; #697 and #687 own those.
- A general borrow / lifetime checker or reference-yielding iterators.
- Index access (`get` / `set`), range removal, or `sort` on these collections.
- Type aliases such as `type Queue<T> = Deque<T>`.

## Required work

- [x] Audit `std::collections::linear` and adjacent collection modules.
- [x] Decide the public namespace:
      `std::collections::deque::Deque<T>` and
      `std::collections::linked_list::LinkedList<T>`; rejected the flat
      `std::collections::Deque<T>` form in ADR-086.
- [ ] Define public struct surfaces for deque / queue / stack / list, with
      private fields and the methods above.
- [ ] Add constructors via associated functions (`Deque::new`,
      `Deque::with_capacity`, `Queue::new`, `Stack::new`,
      `LinkedList::new`) and validate `Deque::new<i32>()` on both targets.
- [ ] Add methods via `impl` blocks, including `Option<T>` empty-state
      results and capacity / reserve where specified.
- [ ] Add the six iterator structs and `IntoIterator`; implement
      `Iterator` (`next` / `collect` / `count`) and
      `IntoIterator<IntoIter = Self>` for each.
- [ ] Add the trait impl plan: `FromIterator` for concrete sources,
      `Clone`, `Debug`, `Eq`. Record the intentional absence of
      `Default` / `Hash` / `Ord` / `Display` / `Copy`.
- [ ] Extend `std/manifest.toml` `[[types]]` with `kind`, `doc`, `methods`,
      and `trait_impls`; update the docs generator so the new types appear
      under **Structs**, not only under module/function lists.
- [ ] Make `deprecated_by` resolve `Owner::member` for the free-function
      migration.
- [ ] Add module pages for `std::collections::deque` and
      `std::collections::linked_list`; update the collections family overview
      to recommend the type API.
- [ ] Deprecate `deque_*` with method-level replacements and an ADR-014
      window; remove `linked_list_*` after in-tree callers migrate.
- [ ] Migrate `tests/fixtures/stdlib_linear/*` and any documented examples to
      the new API; keep the old `Vec<i32>` layout only for the deprecation
      window.
- [ ] Regenerate docs and run the repository gates.

## Acceptance

- [ ] Linear collection APIs have named public structs, and each absent type
      or method has an explicitly documented reason.
- [ ] `Deque<T>` appears under **Structs** in std docs, with method and trait
      impl entries sourced from the manifest registry.
- [ ] The chosen namespace is documented and is not a generic "linear"
      bucket.
- [ ] Method and trait implementation plans are linked to #691, #697, #701,
      and #707.
- [ ] Old helper-style collection APIs have migration paths: `deque_*`
      deprecated with method replacements, `linked_list_*` removed with a
      migration note, `pq_*` explicitly out of scope.
- [ ] `wasm32-gc` and `wasm32` compile / validate / run parity fixtures cover
      the new surfaces.

### Design acceptance

- [x] ADR-086 records the namespace, struct surfaces, constructors, methods,
      iterator split, trait impls, docs data model, migration, stability, and
      implementation dependency order.
- [x] Rejected alternatives (flat namespace, keep `linear`, Queue / Stack
      aliases, Rust-style `&mut` iterators, PriorityQueue in scope, universal
      `FromIterator<I>`, enum `List`) are recorded in ADR-086.

## References

- [ADR-086](../../docs/adr/ADR-086-linear-collection-adts.md) — design record
  (PROPOSED)
- #687 (HashMap / HashSet parity, done)
- #691 (`Iterator` / `IntoIterator` / `FromIterator`, done)
- #697 (`Vec<T>` operation extension)
- #701 (associated function syntax, done)
- #703 (monomorphic API cutover)
- #707 (`Self` return type support, done)
- #709 (trait-first API policy, done)
- #711 (rich stdlib docs; depends on this issue)
- #917 (free-function eradication execution)
- `std/collections/linear.ark`, `std/collections/linked_list.ark`,
  `std/collections/vec.ark`
- `std/manifest.toml`
- `docs/stdlib/generation-schema.md`
- `scripts/gen/generate-docs.py`, `scripts/gen/generate-stdlib-docs.sh`
