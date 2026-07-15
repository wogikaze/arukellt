---
Status: open
Created: 2026-07-15
Updated: 2026-07-16
ID: 816
Parent: 729
Track: compiler-internal
Depends on: "798"
Related: "729, 818, ADR-042, docs/rfcs/005-prelude-compilation-restoration, docs/plans/intrinsic-layer-separation"
Orchestration class: ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 3
Source: ADR-042 out-of-scope item
---

# 816 — Prelude compilation restoration

## Summary

Remove `combine_loaded_and_main_decls_skip_prelude` and make `std/prelude.ark`
bodies real compilable Ark code, as required by the 5-layer separation in
ADR-042. RFC-005 is ACCEPTED; implementation landed the skip removal and
RFC-005 D3 body deferral for CoreOp-bound / intrinsic-shim symbols.

## Scope

- [x] Write a dedicated RFC that defines prelude restoration strategy
      ([RFC-005](../../docs/rfcs/005-prelude-compilation-restoration.md)).
- [x] Remove `combine_loaded_and_main_decls_skip_prelude` from the backend pipeline.
- [x] Defer MIR body lowering for CoreOp-bound and unbound intrinsic shims
      (`mir/lower/stdlib_body_defer.ark`) until #821/#822.
- [x] Keep pure Ark prelude helpers (e.g. `contains_i32`) eligible for body lower.
- [ ] Full Ark fallback bodies for every prelude CoreOp (owned by #821/#822).
- Update `std/manifest.toml` and `docs/current-state.md` accordingly.

## Non-goals

- Do not migrate pure/representation stdlib semantics here (#821/#822).
- Do not set `data/core-ops.toml` `status = "production"` (#818).

## Acceptance

- [x] RFC for prelude restoration is accepted (RFC-005)
- [x] `combine_loaded_and_main_decls_skip_prelude` is removed
- [x] Prelude decls are included in backend combine; CoreOp-bound bodies deferred per RFC-005 D3
- [x] Fake panic stubs are not the restoration strategy (intrinsic shims remain transitional)
- [ ] All prelude functions have real Ark implementations in `std/` modules (#821/#822)
- [ ] `python3 scripts/manager.py verify quick` passes with a selfhost wasm rebuilt from this tree

## Validation commands

- `python3 -m unittest scripts.tests.test_prelude_raw_restoration`
- `python3 scripts/manager.py verify quick`
- `python3 scripts/manager.py docs regenerate`

## References

- `docs/rfcs/005-prelude-compilation-restoration.md`
- `docs/adr/ADR-042-intrinsic-layer-separation.md`
- `docs/plans/intrinsic-layer-separation.md`
- `issues/open/729-intrinsic-layer-separation.md`
- `issues/done/798-adr-042-semantic-operation-registry-migration.md`
- `std/prelude.ark`
- `src/compiler/driver/pipeline_backend.ark`
- `src/compiler/mir/lower/stdlib_body_defer.ark`
