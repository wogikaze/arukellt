---
Status: done
Created: 2026-10-15
Updated: 2026-10-19
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

## Minimal-ish repro (2026-10-15)

`tests/fixtures/stdlib_trait/iterator_query_consumers.ark` and
`iterator_string_callbacks.ark` compile in ~1.5 s under the pinned
bootstrap (`bootstrap/arukellt-selfhost.wasm`) but exceed the 120 s
fixture-parity compile timeout under both the current s2-runtime AND
the pre-change `flat-src-gc-v5/bootstrap-out.wasm` (built 04:41
today).  So the regression window is roughly "after the 2026-10-09
pin, before 2026-10-15 04:41" — candidates in that window include the
#920 deferred-sig binding work and the #922 linear-GC pressure/arena
machinery.  These iterator fixtures exercise exactly the
trait/generic-inference surface that the whole-compiler compile also
stresses.

Practical gate impact: `python3 scripts/manager.py selfhost
fixture-parity` currently reports PASS=1793 FAIL=2 with both failures
being these compile timeouts — pre-existing, not caused by #924–#926.

## Suspects

- Resolver worklist / visited-set over ~500-module graph not making
  progress (frozen RSS argues against allocation-heavy looping).
- Typecheck fixpoint over the full program being super-linear without
  I/O — could also be merely extremely slow rather than a true loop;
  no intermediate progress reporting exists to distinguish.
- A pathological phase that only activates on the full dep graph
  (e.g. trait/mono resolution over the complete overload set).

## Root cause confirmed (2026-10-15, perf + wasm coredump)

Not a module-graph/typecheck bug - the stall is the **linear-memory GC
embedded in s2-runtime.wasm**. The s2 stage is emitted by the Oct-9
pinned bootstrap (bootstrap/arukellt-selfhost.wasm, built from
01c3ec26e), whose emitter predates the collector fixes
(1383275a8 page chains, aabd7c16b grey worklist, dff403b38 /
388bf2099 / 897791cab chain-arena reclaim + hysteresis). Its mark
phase is the old fixpoint rescan: every marked record's payload word
triggers a full ALLOC_HEAD walk.

Evidence:

- perf record on the stalled compile: 99.96% of samples in one wasm
  function. Disassembly shows it is the old mark_candidate (walk
  ALLOC_HEAD records {next@0, lo@4, hi@8, mark@12}); its caller is
  the old fixpoint mark_transitive inside collect.
- -D coredump on -W timeout: the list walks cleanly - **no cycle** -
  but holds **2,670,927 allocation records** (~42 MiB of record
  metadata). Mark cost is O(marked_words x 2.67M records) per collect
  - effectively non-terminating; RSS stays frozen because marking
  never allocates.
- wasm backtrace at interrupt:
  emit_inloop_drain -> emit_function_body -> emit_gc_struct_get ->
  gc_variant_field_is_i32 -> eq -> collect -> mark_transitive. The
  collect fires mid-drain, at the live-set maximum.
- Bisect correction: **s2 built at afe71d68e (parent of 88a3d0434)
  hangs identically** - the earlier post-88a3d0434 boundary was
  s3-vs-s2, i.e. internal-collector generation, not a source change.
  Any s2 produced by the Oct-9 pin hits the cliff once a compile's
  allocation watermark is crossed; which fixture trips it varies.
- The stall point is non-deterministic across inputs (main-body
  lowering on the full fixture; inside emit_inloop_drain on truncated
  variants) because it is wherever the watermark first crosses the
  collect threshold.

This matches the plan doc's prior note
(docs/plans/selfhost-compiler-core-rewrite.md, Phase-3 measurement):
the permanent fix is a pinned bootstrap refresh or the gc-host s2
path - the new collector only reaches s2 via a new pinned emitter.

## Verification with the #922 mark fix (2026-10-19)

`lin_comp_v28` (wasm32 + linear GC emitted by the fixed emitter, the
s3-equivalent artifact, fixpoint-verified byte-identical against its
own recompile) ran `compile src/compiler/main.ark --target wasm32-gc`
to completion of module loading in ~27 s. The O(marked words x
records) mark hang is gone. A control run under the current
s2-runtime (still Oct-9-lineage internal GC) stayed at 100% CPU with
an empty log past 10 min, matching the stale-mark diagnosis.

The now-unmasked next failure is a pre-existing module-resolution
miss set, not the hang: loading reports 16 "failed to load module"
errors for bare `use analysis|dap|lsp|component|mir|resolver|
native_c|wasm` inside nested files and the qualified parent fallback
of `use mir_opt::optimize_fn`. Reproduced identically under the
pinned bootstrap on a synthetic nested package and under the current
s2-runtime on `src/compiler/main/editor.ark` alone - filed as #929.
The flat-src canonical path is unaffected.

Remaining durable fix per the note above: pin a fixed-GC s3 artifact
so the shipped s2-runtime carries the new collector.

The fixed artifact was then installed as the local
`.build/selfhost/arukellt-s2-runtime.wasm` (lin_comp_v28 + the
standard dedupe-export / memory32-normalize postprocessing,
`_reject_invalid_compiler_wasm` clean): the same
`compile src/compiler/main.ark --target wasm32-gc` finishes in
26.2 s wall with only the #929 misses — done-when clause 1 met on
the fixed runtime. Durability still requires the pinned bootstrap
refresh; a `build-compiler` rebuild would revert the runtime to a
stale-collector binary until the pin moves.

## Done when

- `compile src/compiler/main.ark --target wasm32-gc` finishes under
  s2-runtime in bounded time (or the module-graph path is formally
  retired/documented as unsupported with the flat-src path named as
  canonical).
- If it is slowness rather than a loop: a documented budget + timing
  gate so regressions are visible.

## Done evidence (2026-10-19 close)

- Canonical `.build/selfhost/arukellt-s2-runtime.wasm` (sha256
  `b9913678`, produced by `selfhost build-compiler` under the
  refreshed pin `a2803831`) runs `compile src/compiler/main.ark
  --target wasm32-gc` to completion in **27.9 s** — bounded time,
  every day, not just on a hand-installed binary. The run emits
  E0100 cross-module visibility diagnostics (filed as #931) and
  zero module-load misses after the #929 loader fix.
- The completion is a genuine hang fix, not slowness: pre-fix the
  same command stayed at 100% CPU with an empty log past 70 min
  (O(marked words x live records) mark scan). The post-fix mark is
  O(live records) per collection with boundary-correct claiming.
- Durability: the fix lives in the *emitted* collector, so it only
  ships when the runtime binary is rebuilt by a fixed emitter.
  `bootstrap/arukellt-selfhost.wasm` is now the fixpoint-verified
  fixed-emitter artifact (commit `1b8baba76`), making every future
  `build-compiler` output carry it — the regression channel is
  closed at the source.

## Notes

Found while running a deep self-check for #925. The canonical build
path is unaffected; this only blocks direct whole-tree compiles
through module resolution.
