# 583 — Phase 5 prerequisite: retire ARUKELLT_USE_RUST opt-in and purge `arukellt` Rust core consumers

**Status**: open
**Track**: selfhost-retirement
**Depends on**: 559
**Blocks**: 560, 561, 562, 563, 564
**Orchestration class**: source-of-truth-transition

## Why

#560/#561/#562/#563 each STOPped at slice-attempt with the same structural
blocker: `crates/arukellt` (and `crates/ark-lsp` for ark-stdlib) actively
consume Rust core crates via the `ARUKELLT_USE_RUST=1` legacy CLI path
implemented in `crates/arukellt/src/commands.rs` and `crates/arukellt/src/cmd_doc.rs`.

While the consumers exist, the leaf crates ark-driver/ark-mir/ark-wasm/ark-stdlib
cannot be deleted without violating each Phase 5 issue's pre-deletion invariants.

Per #559 the selfhost wrapper (`scripts/run/arukellt-selfhost.sh`) is already
the default execution path, with the Rust legacy reachable only via
`ARUKELLT_USE_RUST=1`. That opt-in was always documented as transitional.

This slice retires the opt-in entirely so Phase 5 deletions become real leaves.

## Pre-deletion invariants

1. 4 canonical selfhost gates PASS at HEAD.
2. `scripts/run/arukellt-selfhost.sh` (selfhost-first wrapper) is the default
   user-facing entry per #559.
3. `ARUKELLT_USE_RUST=1` is the ONLY route through `crates/arukellt/src/commands.rs`
   and the legacy Rust CLI binary; no docs page promises long-term support.

## Acceptance

- [ ] `ARUKELLT_USE_RUST=1` opt-in is retired from `scripts/run/arukellt-selfhost.sh`
  (or the wrapper hard-fails with a clear "use selfhost path" message when set).
- [ ] `crates/arukellt/src/commands.rs` legacy compile/build/run/check/test
  command paths are deleted (or stubbed to return a "use selfhost CLI" error).
- [ ] `crates/arukellt/src/cmd_doc.rs` no longer depends on `ark_stdlib::StdlibManifest`
  (either delete the doc subcommand and route to selfhost-emitted docs JSON, OR
  inline a minimal local TOML reader, OR delete the subcommand entirely if
  selfhost provides equivalent).
- [ ] `crates/arukellt/Cargo.toml` no longer depends on `ark-driver`, `ark-mir`,
  `ark-wasm`, `ark-stdlib`.
- [ ] `cargo build --workspace --exclude ark-llvm` succeeds.
- [ ] All 4 canonical selfhost gates PASS.
- [ ] `rg -n "ark_driver|ark_mir|ark_wasm" crates/arukellt/` returns 0 hits.
- [ ] `rg -n "ark_stdlib" crates/arukellt/` returns 0 hits.
- [ ] `docs/current-state.md` updated to note the opt-in retirement.

## Required verification

1. `cargo build --workspace --exclude ark-llvm`
2. `python3 scripts/manager.py selfhost fixpoint`
3. `python3 scripts/manager.py selfhost fixture-parity`
4. `python3 scripts/manager.py selfhost parity --mode --cli`
5. `python3 scripts/manager.py selfhost diag-parity`
6. `scripts/run/arukellt-selfhost.sh --help` runs via selfhost path (default).

## STOP_IF

- Any selfhost gate regresses to FAIL.
- A required behavior of `arukellt doc` cannot be replicated via selfhost
  within scope — document the gap and stop (do not delete blindly).

## False-done prevention checklist

- Do NOT add SKIPs to `scripts/selfhost/checks.py`.
- Do NOT silence `rg` hits via ignore rules.
- The Rust legacy CLI must be functionally retired, not just renamed.
- The `arukellt` crate may still exist (its selfhost-wasm-runner thin shell),
  but its `Cargo.toml` must not depend on the soon-to-be-deleted Rust core crates.

## PRIMARY paths

- `crates/arukellt/src/commands.rs`
- `crates/arukellt/src/cmd_doc.rs`
- `crates/arukellt/src/native.rs` (if it imports ark_mir/ark_wasm)
- `crates/arukellt/src/main.rs` / `lib.rs` (entry-point dispatch)
- `crates/arukellt/Cargo.toml`
- `scripts/run/arukellt-selfhost.sh` (remove `ARUKELLT_USE_RUST=1` branch
  or hard-error)
- `docs/current-state.md`

## ALLOWED paths (read / minor edit)

- `crates/ark-driver/tests/wit_import_roundtrip.rs` (move/delete if it's the
  last remaining ark_driver consumer)
- `scripts/check/check-panic-audit.sh` (drop dead DIRS entries)

## FORBIDDEN paths

- `src/compiler/*.ark` (no selfhost source edits)
- `crates/ark-{driver,mir,wasm,stdlib,lsp}/src/**` (those are sibling slices)
- `crates/ark-driver/Cargo.toml` etc. for sibling-slice cleanup
- `scripts/selfhost/checks.py`
- Any other open issue file

## Close-note evidence schema

- Files deleted (count + paths)
- `cargo build` tail
- 4 gate logs
- `rg ark_driver|ark_mir|ark_wasm|ark_stdlib crates/arukellt/` → 0 hits
- `arukellt --help` (via wrapper) sample output
