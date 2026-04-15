# T2 (`wasm32-freestanding`) Wasm Emitter Implementation

**Status**: open
**Created**: 2026-04-14
**Updated**: 2026-04-14
**ID**: 501
**Depends on**: none
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 20

## Summary

ADR-020 (`docs/adr/ADR-020-t2-io-surface.md`) defines the T2 I/O surface contract
(Option A: import-based bridge with 1-page linear memory for marshaling).  This issue
tracks the actual emitter implementation so that T2 produces a minimal, WASI-free Wasm
module that can be instantiated in a browser runtime.

This issue was created when issue #382 was audited: all acceptance checkboxes in #382
were incorrectly marked `[x]` (false-done), but the emitter itself was absent.  #382
now tracks only the ADR + docs slice.  The full emitter, fixture, and runtime-proof work
is scoped here.

## Parent note — 2026-04-15

This issue is directly gated by ADR-020, not by #382 as an open issue. #382 remains the
audit/history record for the false-done rollback, but it should not block T2 emitter work.

## Partial progress — 2026-04-15

Wave 1 landed commit `1f33b50d95198ba6dceb109972f7856fbd4cd602`, which added:

- `crates/ark-wasm/src/emit/t2_freestanding.rs`
- target-registry updates for `wasm32-freestanding`
- a first regression path at `tests/fixtures/regression/t2_scaffold.ark`

This issue stays open because the slice did not produce close evidence:

- the validating T2 fixture path is not yet green
- `bash scripts/run/verify-harness.sh --quick`, `--cargo`, and `--fixtures`
  all failed during the slice
- the reported failures included unrelated dirty-tree/workspace problems plus
  pre-existing `ark-wasm` validation breakage when exercising the new path

## Current state

- `crates/ark-target/src/lib.rs`: `wasm32-freestanding` registered, `implemented: false`
- No `crates/ark-wasm/src/emit/t2/` directory
- `docs/target-contract.md` T2 row: "ADR written, emitter not started"
- `docs/adr/ADR-020-t2-io-surface.md` (DECIDED): import-based bridge contract settled

## Scope

### Work items

1. **T2 emitter scaffold** — create `crates/ark-wasm/src/emit/t2/mod.rs`
   - Wasm GC module without any WASI imports
   - Emit the `arukellt_io.write(ptr: i32, len: i32)` + `arukellt_io.flush()` import stubs
     per ADR-020 contract
   - Reuse shared MIR→Wasm lowering from T3 where possible (no fd_write, no WASI runtime)

2. **Target registry update** — flip `implemented: true`, `run_supported: false` (no
   wasmtime runner yet; browser-only execution) in `crates/ark-target/src/lib.rs`

3. **Minimum fixture** — add at least one fixture under `tests/fixtures/t2/` that
   compiles with `--target wasm32-freestanding` and validates (wasmparser) without error

4. **`docs/target-contract.md` update** — T2 row updated to reflect scaffold/smoke tier

5. **Playground wiring** (optional, may be a follow-on) — hook T2 output into
   `crates/ark-playground-wasm` once emitter is stable

## Acceptance

- [ ] `crates/ark-wasm/src/emit/t2/` directory created with functional emitter
- [ ] `wasm32-freestanding` profile has `implemented: true` in `ark-target`
- [ ] At least 1 T2 fixture compiles without error and passes wasmparser validation
- [ ] `docs/target-contract.md` T2 status updated to `scaffold` or higher
- [ ] `bash scripts/run/verify-harness.sh --quick` passes with exit 0

## References

- `docs/adr/ADR-020-t2-io-surface.md` — I/O surface contract (prerequisite)
- `crates/ark-wasm/src/emit/t3_wasm_gc/` — T3 emitter (reference/reuse)
- `crates/ark-target/src/lib.rs` — target registry
- `docs/target-contract.md` — target contract (needs update when emitter ships)
- `issues/open/382-playground-t2-freestanding.md` — parent audit issue

## Estimated scope

Emitter stub + fixture + registry update: ~150–300 lines new Rust + ~20 lines Ark
fixture.  Non-trivial but bounded.
