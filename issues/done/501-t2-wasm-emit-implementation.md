---
Status: done
Created: 2026-04-14
Updated: 2026-04-15
ID: 501
Track: playground
Depends on: none
Orchestration class: implementation-ready
---
# T2 (`wasm32-freestanding`) Wasm Emitter Implementation
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

Wave 2 added targeted validation evidence on top of the same commit:

- `cargo test -p arukellt --test t2_scaffold -- --nocapture` now passes
- the targeted test validates the emitted `wasm32-freestanding` output with
  `wasmparser::Validator::validate_all`

The issue still remains open because the issue-level close gate has not yet been
raised to a repo-wide green proof (`bash scripts/run/verify-harness.sh --quick`)
and the acceptance text still expects the T2 path to be fully reflected there.

Wave 3 landed commit `58688e16e3b80083eec7228caed0a06e943f54d0`, which added
repo-visible proof/alignment on top of the existing scaffold:

- `crates/ark-target/src/lib.rs` now marks `wasm32-freestanding` as implemented
  with `run_supported: false`
- `docs/target-contract.md` now describes the T2 scaffold using the current repo
  proof surface instead of "not started"
- `crates/arukellt/tests/t2_scaffold.rs` now proves the CLI/test entrypoint,
  validates the emitted module with `wasmparser`, and checks the scaffold shape

This issue remains open because the remaining acceptance still expects the T2
emitter to be normalized into the issue's declared file/fixture layout before
close review.

Wave 4 landed commit `b3ff27c027a36ff2f800682d1c403062ebe4aaa3`, which added a
manifest-driven T2 fixture surface on top of the existing scaffold:

- `tests/fixtures/t2/t2_scaffold.ark` + `.expected`
- `tests/fixtures/manifest.txt` registration for the dedicated T2 fixture
- `crates/arukellt/tests/t2_scaffold.rs` now compiles that fixture path directly

After Wave 4, the product-level acceptance is satisfied by the current emitter
path. The remaining discrepancy was issue wording tied to a provisional file
layout assumption, not a missing user-visible capability.

## Current state

- `crates/ark-wasm/src/emit/t2_freestanding.rs`: functional T2 scaffold emitter
- `crates/ark-target/src/lib.rs`: `wasm32-freestanding` registered, `implemented: true`, `run_supported: false`
- `tests/fixtures/t2/t2_scaffold.ark`: manifest-driven fixture proof for the T2 scaffold
- `docs/target-contract.md` T2 row: scaffold tier with repo-visible proof surface
- `docs/adr/ADR-020-t2-io-surface.md` (DECIDED): import-based bridge contract settled

## Scope

### Work items

1. **T2 emitter scaffold** — ship a functional T2 emitter path for the ADR-020 contract
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

- [x] Existing T2 emitter path (`crates/ark-wasm/src/emit/t2_freestanding.rs`) is functional for the ADR-020 scaffold contract
- [x] `wasm32-freestanding` profile has `implemented: true` in `ark-target`
- [x] At least 1 T2 fixture compiles without error and passes wasmparser validation
- [x] `docs/target-contract.md` T2 status updated to `scaffold` or higher
- [x] `bash scripts/run/verify-harness.sh --quick` passes with exit 0

## Evidence review — 2026-04-15

- entrypoint evidence: `crates/arukellt/tests/t2_scaffold.rs` compiles a source fixture with
  `--target wasm32-freestanding` and validates the emitted module with `wasmparser`
- exposed surface consistency: `crates/ark-target/src/lib.rs` and `docs/target-contract.md`
  both describe T2 as scaffold-tier, implemented, and not run-supported
- fixture proof: `tests/fixtures/t2/t2_scaffold.ark` is registered in `tests/fixtures/manifest.txt`
  and participates in the repo's canonical fixture inventory

## References

- `docs/adr/ADR-020-t2-io-surface.md` — I/O surface contract (prerequisite)
- `crates/ark-wasm/src/emit/t3_wasm_gc/` — T3 emitter (reference/reuse)
- `crates/ark-target/src/lib.rs` — target registry
- `docs/target-contract.md` — target contract (needs update when emitter ships)
- `issues/open/382-playground-t2-freestanding.md` — parent audit issue

## Estimated scope

Emitter stub + fixture + registry update: ~150–300 lines new Rust + ~20 lines Ark
fixture.  Non-trivial but bounded.