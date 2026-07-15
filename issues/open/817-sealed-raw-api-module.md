---
Status: open
Created: 2026-07-15
Updated: 2026-07-16
ID: 817
Parent: 729
Track: compiler-internal
Depends on: "798"
Related: "729, 818, ADR-042, docs/rfcs/006-sealed-raw-api, docs/plans/intrinsic-layer-separation"
Orchestration class: ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 3
Source: ADR-042 out-of-scope item
---

# 817 — Sealed raw API module for Vec/String internal representation

## Summary

Define a sealed raw API module that exposes the internal representation of
`Vec` and `String` only to the standard library, as required by ADR-042.
RFC-006 is ACCEPTED and selects `core::raw` (load path `std::core::raw`).

## Scope

- [x] Dedicated RFC selecting the sealed module name (`core::raw`) and surface
      ([RFC-006](../../docs/rfcs/006-sealed-raw-api.md)).
- [x] Implement `std/core/raw.ark` with the minimal raw array/string surface.
- [x] Visibility enforcement: user imports of `std::core::raw` / `core::raw`
      rejected via `importer_is_stdlib` gate in `module_graph.ark`.
- [x] Route `std/collections/vec.ark` / `string.ark` construction and selected
      accessors through `std::core::raw`.
- [x] Remove dual `intrinsic_*_lm.ark` pairs (merged into parent dispatchers).
- [ ] Full generic Vec/String representation migration (#822).
- [ ] Runtime differential coverage across GC and LM for every raw op.

## Non-goals

- Do not expose the raw API to user code.
- Do not migrate semantic stdlib operations (split/join/…) here (#822).

## Acceptance

- [x] RFC for sealed raw API is accepted (RFC-006)
- [x] Sealed raw API module is created; user import path is rejected in loader
- [x] Vec/String stdlib entry points begin using the sealed raw API
- [x] GC/LM dual `*_lm.ark` files are removed (helpers merged into parents)
- [ ] Differential tests pass across GC and LM targets for raw ops
- [ ] `python3 scripts/manager.py verify quick` passes with a selfhost wasm rebuilt from this tree

## Validation commands

- `python3 -m unittest scripts.tests.test_prelude_raw_restoration`
- `python3 scripts/manager.py verify quick`
- `python3 scripts/manager.py docs regenerate`

## References

- `docs/rfcs/006-sealed-raw-api.md`
- `docs/adr/ADR-042-intrinsic-layer-separation.md`
- `docs/plans/intrinsic-layer-separation.md`
- `issues/open/729-intrinsic-layer-separation.md`
- `issues/done/798-adr-042-semantic-operation-registry-migration.md`
- `std/core/raw.ark`
- `std/collections/vec.ark`
- `std/collections/string.ark`
- `src/compiler/loader/module_sealed_raw.ark`
