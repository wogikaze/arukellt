# native S3 vs wasmtime S3 function-count drift (+10)

## Summary
With the same flat overlay workspace and `wasm32`/`wasi-p1` profile, native
executor S3 and wasmtime-hosted S2-runtime S3 both validate but are not
byte-identical. Native emits 8441 functions vs wasmtime 8431 (+10). S2 from the
bootstrap chain is larger still and does not yet match either S3 (fixpoint not
reached on this source revision).

## Observed
- native S3 size 2466727; wasmtime S3 size 2465899 (delta +828)
- section deltas concentrated in functions / exports / code / data
- both validate with `wasm-tools validate`

## Acceptance
- Identify the 10 extra (or missing) functions and owning lowering difference
- native S3 byte-equals wasmtime S3 under identical profile and overlay
- then re-check bootstrap S2 vs S3 fixpoint convergence
