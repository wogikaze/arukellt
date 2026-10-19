---
Status: open
Created: 2026-10-19
Updated: 2026-10-19
ID: 929
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "#927 verification — with the linear-GC mark fix, module-graph compile of src/compiler/main.ark completes in ~27 s and surfaces these pre-existing load misses"
Related: "#927, #851"
---

# 929 — module-graph compile: bare `use X` from nested files and qualified-import parent fallback miss package-root modules

## Summary

Compiling `src/compiler/main.ark` through the real module-resolution
path (not the flat-src overlay) finishes module loading and then
reports 16 load misses:

    module loading error(s)
    failed to load module `analysis`
    failed to load module `dap`
    failed to load module `lsp`
    failed to load module `component`
    failed to load module `mir`
    failed to load module `component`
    failed to load module `resolver`
    failed to load module `mir`
    failed to load module `component`
    failed to load module `native_c`
    failed to load module `wasm`
    failed to load module `resolver`  (x4)
    failed to load module `mir_opt::optimize_fn`  (x2)

Two related resolution gaps, both in
`src/compiler/loader/module_graph.ark`:

1. Bare `use X` (no `::`) resolves only against the importer's
   directory (`module_base_dir_for_import` returns `current_dir`).
   `src/compiler/main/editor.ark` has `use analysis` / `use dap` /
   `use lsp`; the candidates `src/compiler/main/{analysis,dap,lsp}.ark`
   do not exist, so the edge fails instead of resolving
   `src/compiler/analysis.ark` at the package root. The same pattern
   exists in `mir_lower.ark` (`use mir`), `driver/emit.ark` and
   `emitter.ark` (`use wasm`), and other nested files. The misses were
   invisible until now because the flat-src overlay makes every file a
   sibling, and the #927 GC hang killed the compile before the error
   report.
2. Qualified-import parent fallback uses the wrong base:
   `use mir_opt::optimize_fn` pushes parent `mir_opt`, whose fallback
   base is `module_base_dir_for_import("mir_opt", fallback_dir, ..)` —
   the bare name takes `fallback_dir` (importer's dir), so
   `src/compiler/mir_opt/mod.ark` at the package root is never tried.

## Evidence (2026-10-19)

- `lin_comp_v28` (wasm32+linear selfhost compiler carrying the #922
  mark fix): `compile src/compiler/main.ark --target wasm32-gc
  --wasi-version wasi-p2` exits with the 16 errors above in ~27 s
  (the #927 hang — O(marked words x live records) — is gone).
- Same misses under the current s2-runtime on a smaller graph:
  `compile src/compiler/main/editor.ark` prints `analysis`, `dap`,
  `lsp` errors in <1 s.
- Pre-existing semantics, not a #922 regression: a synthetic package
  (`pkg/sub/user.ark` with bare `use helper` where `helper.ark` sits
  at the package root) produces the identical "failed to load module
  `helper`" under the pinned bootstrap compiler
  (`bootstrap/arukellt-selfhost.wasm`, 2026-10-09).

## Expected

When the importer-relative candidates miss, local module resolution
should also try the package root (`root_dir`) before reporting an
error — for bare `use X` and for the parent fallback of qualified
imports alike. This is additive: sibling-first keeps current
resolution order for sources that already resolve.

## Acceptance

- [ ] `compile src/compiler/main.ark` through the module-graph path
      produces no "failed to load module" errors.
- [ ] Sibling resolution order is unchanged (importer's dir still
      wins when both locations provide the module).
- [ ] A module-run fixture covers bare `use` resolution from a nested
      package file to a package-root module.
