---
Status: open
Created: 2026-07-15
Updated: 2026-07-15
ID: 816
Track: compiler-internal
Depends on: "729"
Related: "798, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: blocked
Orchestration upstream: "prelude-restoration-rfc"
Blocks v{N}: none
Priority: 3
Source: ADR-042 out-of-scope item
---

# 816 — Prelude compilation restoration

## Summary

Remove `combine_loaded_and_main_decls_skip_prelude` and make `std/prelude.ark`
bodies real compilable Ark code, as required by the 5-layer separation in
ADR-042. This work is intentionally out of scope for the main #729 epic until a
separate RFC is accepted.

## Scope

- Write a dedicated RFC that defines prelude restoration strategy, including
  ordering and backwards compatibility with existing `std/manifest.toml`.
- Remove `combine_loaded_and_main_decls_skip_prelude` from the backend pipeline.
- Convert prelude stub bodies into real Ark implementations (or thin wrappers
  to `std::` modules).
- Ensure all prelude functions have real implementations in `std/` modules.
- Update `std/prelude.ark` to remove fake declarations.
- Update `std/manifest.toml` and `docs/current-state.md` accordingly.

## Non-goals

- Do not implement prelude restoration before the RFC is accepted.
- Do not change prelude semantic surface before the main registry migration in
  #798 is complete.

## Acceptance

- [ ] RFC for prelude restoration is accepted
- [ ] `combine_loaded_and_main_decls_skip_prelude` is removed
- [ ] Prelude bodies are compiled by the backend
- [ ] Fake stub bodies in `std/prelude.ark` are removed
- [ ] All prelude functions have real implementations in `std/` modules
- [ ] `python3 scripts/manager.py verify quick` passes with prelude compiled

## Validation commands

- `python3 scripts/manager.py verify quick`
- `python3 scripts/manager.py docs regenerate`

## References

- `docs/adr/ADR-042-intrinsic-layer-separation.md`
- `docs/plans/intrinsic-layer-separation.md`
- `issues/open/729-intrinsic-layer-separation.md`
- `issues/open/798-adr-042-semantic-operation-registry-migration.md`
- `std/prelude.ark`
- `src/compiler/driver/pipeline_backend.ark`
