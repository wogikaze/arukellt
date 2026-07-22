# native S3 vs wasmtime S3 — resolved under clean nodump conditions

## Summary
With identical flat overlay, `wasm32`/`wasi-p1`, and empty separate caches,
**native executor and wasmtime S2-runtime produce byte-identical S3** when
`--dump-phases` is **not** used.

Earlier +10 function drift (8441 vs 8431) was **not reproducible** with clean
caches on this revision (stale/mismatched prior artifacts).

## Critical finding: `--dump-phases mir` mutated emit

`mir/dump_core.ark` `dump_mir` used to call `mir_compute_dominance` and
`mir_ssa_rename_module` **in place** before emit. That changed Wasm output:

| Mode | SHA-256 prefix | Size |
|------|----------------|------|
| no dump (canonical) | `7ebfc6ff…` | 2469094 |
| `--dump-phases mir` | `e34fc3f0…` | 2469104 |

Native and wasmtime stayed equal **within** each mode. Fix: dump must not
mutate the MIR module (see commit on `dump_core.ark`).

## Bootstrap S2 vs S3 (separate from native drift)

Pre-promotion bootstrap `arukellt-s2.wasm` had 9268 funcs vs S3 8441 with the
**same** `source_fingerprint`. Export diff: ~838 S2-only (SIMD/WIT/DIAG/mir/…)
and 11 S3-only (mostly `String::*` naming). S3→S4 (heap-patched) was
byte-equal → thin compiler is at fixpoint; fat bootstrap S2 was not.

After promoting nodump S3 to S2 in the recovery build dir, S2==native S3.

## Canonical hashes (nodump, this worktree build dir)

- S2 / wasmtime S3 / native S3: `7ebfc6ffc2324a886a7ce7946d84c13846317dec63f87154e3a655e27c8b84a4`
- Tools: `scripts/debug/compare-s3-mir-functions.py`
- Artifacts: `.build-native-recovery/selfhost/native/function-diff/`

## Acceptance
- [x] clean-cache native == wasmtime (nodump)
- [x] identify dump-phase emit mutation
- [x] dump_mir made non-mutating
- [ ] rebuild selfhost compiler wasm so dump fix is in s2/s3 binaries (follow-up)
- [ ] land promoted S2 via normal `selfhost fixpoint --build` on master when ready


## Final native-executor receipt (after dump fix + S2 promote)

- `byte equality: True`
- `deterministic: True`
- S2/S3 SHA-256: `064981f1ef3ed4ac7f69d1102bbe686f80ffef17b5f0c96b5058baa2e7de2e9d`
- `exit_code: 1` solely due to RSS gate (`executor_peak_rss_bytes` ≈ 12.21 GiB > 2.4 GiB)
- warm executor ≈ 579s (dump/rebuild cold path; prior warm ~43–52s without full regen)
