---
Status: done
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

- [x] `compile src/compiler/main.ark` through the module-graph path
      produces no "failed to load module" errors.
- [x] Sibling resolution order is unchanged (importer's dir still
      wins when both locations provide the module).
- [x] A module-run fixture covers bare `use` resolution from a nested
      package file to a package-root module.

## Fix (implemented 2026-10-19)

`src/compiler/loader/module_graph.ark`: new
`try_load_at_package_root` helper runs after the importer-relative
(or qualified `root_dir`) candidates miss — for bare `use X` directly,
and for the parent-module fallback of qualified imports (`use
a::b::sym` → parent `a::b`). It returns early when `base_dir ==
root_dir`, treats already-loaded/loading root candidates as resolved
(preserving the circular-import error the dedup check pushes), and
otherwise loads the module with `root_dir` as its base so recursive
imports resolve from the real file location.

Sibling order is preserved: the root retry only fires after every
importer-relative candidate missed, and the parent fallback still
prefers `module_base_dir_for_import(parent, importer_dir, ..)` before
the root retry.

Regression fixture:
`tests/fixtures/modules/bare_use_root_fallback/` — `sub/user.ark` does
`use helper` (sibling `sub/helper.ark` returns 1, package-root
`helper.ark` returns 100 — expected output 8 proves the sibling won)
and `use root_only` (only resolvable via the package-root fallback).
A failing run prints 107 or reports "failed to load module
`root_only`".

## Done evidence (2026-10-19 close)

Verified under `lin_comp_v29` and the canonical
`.build/selfhost/arukellt-s2-runtime.wasm` (sha256 `b9913678`,
byte-identical to the stage-2 `bootstrap-out.wasm` built by the
refreshed pin — two producers, one output):

- `compile src/compiler/main.ark --target wasm32-gc` finishes in
  27.9 s with **zero** "failed to load module" errors; the remaining
  diagnostics are E0100 cross-module visibility errors filed as #931.
- `compile src/compiler/main/editor.ark` likewise reports no
  analysis/dap/lsp misses.
- `module-run:modules/bare_use_root_fallback/main.ark` prints `8`
  under the new loader (sibling `sub/helper.ark` contributes 1, not
  the root's 100 — order preserved; `root_only` resolves via the new
  fallback). Under the previous loader binary (`lin_comp_v28`) the
  same fixture fails — module loading error `root_only` — as expected.
- `verify lane` PASS (fmt / lint / comment / semantic-debt /
  code-quality ratchets) after `arukellt fmt` normalization.
