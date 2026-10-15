---
Status: open
Created: 2026-10-15
Updated: 2026-10-15
ID: 927
Track: compiler
Depends on: ""
Related: "#922, #924"
Orchestration class: investigation
Blocks v4 exit: False
---

# Selfhost compiler hangs compiling src/compiler/main.ark via module graph

## Summary

`compile src/compiler/main.ark --target wasm32-gc` under the current s2
selfhost wasm never finishes (observed >70 min at 100% CPU, stable RSS).
The canonical flat-src build path
(`scripts/manager.py selfhost build-compiler`, pinned bootstrap ->
flat-src -> s2) completes in ~30-50s, so this only reproduces through
the in-guest module-resolution path — a file-per-module walk of the
whole compiler source tree.

## Evidence (2026-10-15)

- s2-runtime (built 18:22 with #924+#925 fixes): `compile src/compiler/main.ark`
  still running at 75+ min, rss frozen at 158 MB, killed manually.
- Pre-change binary `.build/selfhost/flat-src-gc-v5/bootstrap-out.wasm`
  (built 04:41, before today's parser changes): same compile, timed out
  at 480 s -> the hang predates #924/#925.
- WASMTIME_LOG=wasmtime_wasi=trace: all source file reads finish ~16 s
  in, then zero I/O while CPU stays pinned -> post-load (resolve /
  typecheck / lower) compute loop or pathological work.
- `--dump-phases tokens` / `ast`: main.ark's own tokens+AST dump fine;
  `--dump-phases hir` never appears within 300 s -> hang is before HIR,
  i.e. inside module-graph resolve or typecheck.
- `check src/compiler/main.ark` (no emit): also >600 s timeout.
- `lint --local` on all 1931 `src/compiler/*.ark` files: every file
  parses cleanly in <10 s -> not a parse-loop on any single file.
- Trivial single-file programs compile in <1 s; multi-module fixtures
  compile fine -> scale- or shape-specific.

## Suspects

- Resolver worklist / visited-set over ~500-module graph not making
  progress (frozen RSS argues against allocation-heavy looping).
- Typecheck fixpoint over the full program being super-linear without
  I/O — could also be merely extremely slow rather than a true loop;
  no intermediate progress reporting exists to distinguish.
- A pathological phase that only activates on the full dep graph
  (e.g. trait/mono resolution over the complete overload set).

## Done when

- `compile src/compiler/main.ark --target wasm32-gc` finishes under
  s2-runtime in bounded time (or the module-graph path is formally
  retired/documented as unsupported with the flat-src path named as
  canonical).
- If it is slowness rather than a loop: a documented budget + timing
  gate so regressions are visible.

## Notes

Found while running a deep self-check for #925. The canonical build
path is unaffected; this only blocks direct whole-tree compiles
through module resolution.
