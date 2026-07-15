---
Status: open
Created: 2026-07-15
Updated: 2026-07-15
ID: 817
Parent: 729
Track: compiler-internal
Depends on: "798"
Related: "729, 818, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: blocked
Orchestration upstream: "sealed-raw-api-rfc"
Blocks v{N}: none
Priority: 3
Source: ADR-042 out-of-scope item
---

# 817 — Sealed raw API module for Vec/String internal representation

## Summary

Define a sealed raw API module that exposes the internal representation of
`Vec` and `String` only to the standard library, as required by ADR-042. This
is a separate child of #729, outside #798's dispatch-spine scope, and
remains blocked until a dedicated RFC selects the module name and surface.

## Scope

- Write a dedicated RFC that selects the sealed raw API module name
  (candidates: `core::raw`, `core::rt`, `core::intrinsics`) and defines its
  surface.
- Implement the sealed module with visibility enforcement that prevents user
  code from accessing it.
- Provide the minimal raw operations needed by `Vec`/`String` stdlib:
  `raw_array_new<T>`, `raw_array_len<T>`, `raw_array_get_unchecked<T>`,
  `raw_array_set_unchecked<T>`, `raw_array_grow<T>`, plus raw string storage
  accessors.
- Refactor `Vec`/`String` implementations to use only the sealed raw API.
- Absorb GC/LM representation differences into the raw API layer.
- Remove dual `intrinsic_*_gc.ark` / `intrinsic_*_lm.ark` files as part of this
  migration.

## Non-goals

- Do not select the sealed module name or surface before the RFC is accepted.
- Do not expose the raw API to user code.
- Do not begin implementation before #798 (registry migration) is complete.

## Acceptance

- [ ] RFC for sealed raw API is accepted
- [ ] Sealed raw API module is created and accessible only to stdlib
- [ ] `Vec`/`String` implementations use only the sealed raw API
- [ ] GC/LM representation differences are isolated to the raw API layer
- [ ] Dual `intrinsic_*_gc.ark` / `intrinsic_*_lm.ark` files are removed
- [ ] Differential tests pass across GC and LM targets
- [ ] `python3 scripts/manager.py verify quick` passes

## Validation commands

- `python3 scripts/manager.py verify quick`
- `python3 scripts/manager.py docs regenerate`

## References

- `docs/adr/ADR-042-intrinsic-layer-separation.md`
- `docs/plans/intrinsic-layer-separation.md`
- `issues/open/729-intrinsic-layer-separation.md`
- `issues/done/798-adr-042-semantic-operation-registry-migration.md`
- `std/collections/vec.ark`
- `std/text/string.ark`
- `src/compiler/wasm/intrinsic_*_gc.ark`
- `src/compiler/wasm/intrinsic_*_lm.ark`
