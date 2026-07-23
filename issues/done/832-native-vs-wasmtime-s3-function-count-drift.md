# native S3 vs wasmtime S3 — resolved under clean nodump conditions

## Summary
With identical flat overlay, `wasm32`/`wasi-p1`, and empty separate caches,
**native executor and wasmtime S2-runtime produce byte-identical S3** when
`--dump-phases` is **not** used.

Earlier +10 function drift (8441 vs 8431) was **not reproducible** with clean
caches on this revision (stale/mismatched prior artifacts).

## Critical finding: `--dump-phases mir` mutated emit

`mir/dump_core.ark` `dump_mir` used to call `mir_compute_dominance` and
`mir_ssa_rename_module` **in place** before emit. That changed Wasm output.

Native and wasmtime stayed equal **within** each mode. Fix: dump must not
mutate the MIR module (`850d705a` and successors).

## Bootstrap S2 vs S3 (separate from native drift)

Fat bootstrap `arukellt-s2.wasm` is not byte-equal to thin S3. Promote thin
native/wasmtime S3 into the recovery build-dir S2 for the equality peer.
`selfhost fixpoint --build` on wasm32-gc still rejects stage-3 via
`emit_native_c_module` validation (pre-existing GC emit issue; tracked
separately from this native wasm32 equality lane).

## Canonical hashes (nodump, recovery build dir)

- S2 / native S3: `4975cd51501ff76e3696ac2f3b3e4e66bc3d53b06919c349dcefb4d71675562a`
- Tools: `scripts/debug/compare-s3-mir-functions.py`

## Acceptance
- [x] clean-cache native == wasmtime (nodump)
- [x] identify dump-phase emit mutation
- [x] dump_mir made non-mutating
- [x] rebuild selfhost compiler wasm so dump fix is in s2/s3 binaries
      (build-compiler + promote thin S3 → S2; native-executor equality green)
- [ ] land promoted S2 via normal `selfhost fixpoint --build` on master when
      wasm32-gc `emit_native_c_module` validation is fixed (follow-up)

## Final native-executor receipt (operational / `--allow-high-rss`)

- `byte equality: True`
- `deterministic: True`
- S2/S3 SHA-256: `4975cd51501ff76e3696ac2f3b3e4e66bc3d53b06919c349dcefb4d71675562a`
- `exit_code: 0` with `--allow-high-rss` (RSS ~12.3 GiB; warm ~228s)
- Strict GC=1 lane: RSS ~1.55 GiB (under 2.4), warm ~480s (over 5 min) — experimental not yet
