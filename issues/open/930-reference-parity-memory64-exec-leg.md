---
Status: open
Created: 2026-10-19
Updated: 2026-10-19
ID: 930
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: "pinned bootstrap refresh audit — selfhost-fixture-test.py
  --reference (2026-10-19)"
Related: "#922, #928, ADR-029"
---

# 930 — reference-parity execution leg cannot package memory64 cores;
# 6 run fixtures emit invalid wasm32-gc/memory64 modules

## Summary

Two findings from the pinned-refresh fixture audit
(`python3 scripts/run/selfhost-fixture-test.py --reference`),
orthogonal to the refresh candidate itself:

1. The dual-run oracle compiles every `run:` fixture as
   `wasm32-gc`/`wasi-p2` with the default **memory64** memory, then
   packages each core via `wasm-tools component embed` +
   `component new --reject-legacy-names --realloc-via-memory-grow`.
   Every package step fails under wasm-tools 1.252.0:

       canonical option `realloc` uses a core function with an
       incorrect signature

   The emitted core exports `cm32p2_realloc` as
   `(i32,i32,i32,i32)->i32` while its memory is `i64`; the canonical
   ABI expects an `i64` realloc for a memory64 module. Pinned and
   current legs fail identically (verified on
   `control/if_basic.ark`), so the entire execution-comparison leg
   produces no signal — compile and validate legs are unaffected.
   Either the parity harness should compile fixtures memory32 (the
   width the standard `run:` gate uses), or the cm32p2 realloc
   export needs a memory-width-matching signature.

2. Six `run:` fixtures emit wasm that fails `wasm-tools validate`
   under `wasm32-gc`/memory64:

       scalar/conversion.ark            func 16: expected i32, found i64
       scalar/sign_extend.ark           func 16: expected i32, found i64
       stdlib_fs/exists_probe_contract  func 16: expected i32, found i64
       stdlib_io/exists_missing.ark     func 17: expected i32, found i64
       stdlib_wasm/memory_ops.ark       func 16: expected i32, found i64
       stdlib_trait/ord_sort_vec_string func 28/20: expected
           (ref null $type), found (ref null $type)

   All six reproduce byte-for-byte (same offset) under the pre-#922
   emitter (`.build/selfhost/arukellt-s2-post891.wasm`, 2026-09-28),
   so they are pre-existing emitter bugs in the wasm32-gc/memory64
   path — not #922 regressions and not refresh-candidate drift.
   `ord_sort_vec_string` may share the #928 generic-callee-dispatch
   family (same `ref null $type` mismatch shape); the i32/i64
   failures look like a memory64 lowering leak in conversion/fs/io
   emit paths.

## Evidence (2026-10-19)

- `--reference` run: `PASS=0 FAIL=1796 SKIP=0 (wasm-invalid=6)` —
  1790 uniform packaging failures + 6 validation failures above.
- Manual repro: `component new` fails on both pinned- and
  v28-compiled `if_basic` cores; both export i32 `cm32p2_realloc`
  with `(memory i64 1)`.

## Acceptance

- [ ] Reference-parity execution leg produces real output
      comparison again (fixture memory width or realloc signature
      fixed).
- [ ] The six listed fixtures pass `wasm-tools validate` under
      wasm32-gc/memory64 emit.
