# native S3 vs wasmtime S3 function-count drift (+10)

## Summary
With the same flat overlay workspace and `wasm32`/`wasi-p1` profile, native
executor S3 and wasmtime-hosted S2-runtime S3 both validate but are not
byte-identical. Native emits 8441 functions vs wasmtime 8431 (+10). S2 from the
bootstrap chain is larger still and does not yet match either S3 (fixpoint not
reached on this source revision).

## Observed
- native S3 size 2469094; wasmtime S3 size 2465899 (delta +3195); funcs 8441 vs 8431
- section deltas concentrated in functions / exports / code / data
- both validate with `wasm-tools validate`

## Acceptance
- Identify the 10 extra (or missing) functions and owning lowering difference
- native S3 byte-equals wasmtime S3 under identical profile and overlay
- then re-check bootstrap S2 vs S3 fixpoint convergence

## Latest measurement (wave/native-cpp-recovery)
- native S3 SHA-256: `7ebfc6ffc2324a886a7ce7946d84c13846317dec63f87154e3a655e27c8b84a4`
- wasmtime S3 SHA-256: `66eb18f38f189274e3bb4ff31155e733b1b9523a2ed58ad92e9b33da5c34b454`
- bootstrap S2 SHA-256: `2abbda53e60c377fe0681b7dd05e0f9c6f7739998fc33d9ba05caeed662355ef` (funcs 9268; not yet a fixpoint peer of either S3)
- Independent `/usr/bin/time -v` max RSS for native full S3 (arena default): ~12.3 GiB
- Exact GC exists behind `ARUKELLT_NATIVE_GC=1` but auto-collect still SIGSEGV on full compiler (shadow-stack incomplete)


## Clean-cache recheck (2026-07-22, wave/native-cpp-recovery)

Under identical flat overlay, `wasm32`/`wasi-p1`, and **empty separate caches**,
native executor and wasmtime-hosted S2-runtime produce:

- reachability: `9351 -> 8441` on both
- MIR function multiset: identical (ordered, Counter delta 0)
- S3 SHA-256: `e34fc3f083d42d27098c8b40fea18be07b3aaa43b48a75e475b342e52a0d4531` (both)
- Wasm funcs/codes: 8441 / 8441 (both)

Conclusion: the earlier +10 (8441 vs 8431) is **not reproducible** with clean
caches on this revision. Treat prior artifact drift as stale-cache / mismatched
overlay or source fingerprint until a clean-cache reproduction exists.

Artifacts: `.build-native-recovery/selfhost/native/function-diff/`
Tool: `scripts/debug/compare-s3-mir-functions.py`
