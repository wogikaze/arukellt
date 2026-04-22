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

## Status note 2025-XX (impl-selfhost-retirement attempt — STOPPED)

**Outcome**: STOP_IF triggered ("Any selfhost gate regresses to FAIL"). The
slice cannot be completed under the current FORBIDDEN_PATHS.

### Verified baseline (HEAD 556046b8, fresh build)

- `cargo build --workspace --exclude ark-llvm` → PASS
- `python3 scripts/manager.py selfhost fixpoint --build` → PASS
- `python3 scripts/manager.py selfhost diag-parity` → PASS
- `python3 scripts/manager.py selfhost parity --mode --cli` → PASS

### Structural blocker

All 4 canonical selfhost gates implemented in `scripts/selfhost/checks.py`
(FORBIDDEN to edit by this slice) require the Rust `target/debug/arukellt`
binary as the **parity baseline**:

- `_find_arukellt(root)` is hard-required by `run_fixpoint`,
  `run_fixture_parity`, `run_diag_parity`, and `_run_cli_parity`.
- `run_fixpoint` calls `arukellt compile src/compiler/main.ark -o
  .build/selfhost/arukellt-s1.wasm` as Stage 1 — there is no committed
  bootstrap wasm (`.build/`, `.bootstrap-build/` are both `.gitignore`d).
- `run_diag_parity` requires `pass_count >= 10` AND requires the Rust
  binary's `arukellt check fixture.ark` output to contain each fixture's
  `.diag` pattern. A stubbed/erroring binary causes every fixture to be
  classified `skip: (Rust: pattern not found)` → 0 pass → FAIL.
- `_run_cli_parity` compares Rust-binary `--version`/`--help` output
  byte-for-byte against the selfhost wasm.

### Why the obvious workarounds also stop

Three escape hatches were considered and rejected within scope:

1. **Delete the Rust binary entirely** → `_find_arukellt` returns `None` →
   diag-parity / cli-parity return rc=1 → gates FAIL. Also breaks
   `run_fixpoint` Stage 1 on fresh clones (no bootstrap wasm in repo).

2. **Stub the Rust binary to print "use selfhost" and exit nonzero** →
   compile/check return nonzero with no diagnostic content → diag-parity
   skips all → `pass_count < 10` → FAIL. cli-parity `--version`/`--help`
   outputs differ → FAIL.

3. **Replace the Rust binary with a wasmtime shim that exec's the
   selfhost wasm** → Cargo deps could be dropped and gates would
   *trivially* pass (self-vs-self comparison), BUT (a) makes "parity"
   gates structurally meaningless without updating `checks.py` to
   reflect the new contract, (b) requires a committed bootstrap selfhost
   wasm because `.build/selfhost/arukellt-s1.wasm` is currently produced
   *by* the Rust binary on fresh clones, (c) is silent renaming, which
   FALSE_DONE_PREVENTION forbids: "The Rust legacy CLI must be
   functionally retired, not just renamed."

### What unblocks this slice

This slice needs to land **as part of**, not before, a coordinated change
to the parity-gate contract. Concretely, one of:

- **Option A — gate redesign**: a sibling slice (allowed to edit
  `scripts/selfhost/checks.py`) replaces "Rust-vs-selfhost" parity with
  "selfhost-vs-pinned-reference-wasm" or "selfhost determinism only".
  Then this slice can delete the Rust binary cleanly.

- **Option B — committed bootstrap**: a sibling slice commits a frozen
  `arukellt-s1.wasm` (or a checksum-pinned download recipe) to the repo
  so the gates can run without the Rust binary having to build it.
  Then this slice can replace the Rust binary with a wasmtime shim.

- **Option C — combined slice**: lift the FORBIDDEN restriction on
  `scripts/selfhost/checks.py` for a single follow-up issue that owns
  both the gate redesign and the Rust-CLI retirement atomically.

### Recommendation

File a follow-up issue that bundles this work order with the gate
contract update (Option A is cleanest because it preserves the
"selfhost has not regressed" signal without keeping a parallel Rust
implementation alive). Until then, #560/#561/#562/#563/#564 remain
blocked on the same root cause this slice was created to resolve.

### Evidence

```text
$ rg -n 'use ark_(driver|mir|wasm|stdlib)' crates/arukellt/
crates/arukellt/src/cmd_doc.rs:3:use ark_stdlib::{ManifestFunction, ManifestModule, StdlibManifest};
crates/arukellt/src/commands.rs:7:use ark_driver::{MirSelection, OptLevel, Session};
crates/arukellt/src/commands.rs:9:use ark_mir::mir::{MirModule, MirStmt, Operand, Rvalue};
crates/arukellt/src/commands.rs:11:use ark_wasm::component::{WitDocument, WitFunction, WitType, parse_wit};
crates/arukellt/src/commands.rs:1563:    use ark_wasm::component::{WrapError, compose_components};
crates/arukellt/src/native.rs:94:    let mir = ark_mir::lower::lower_to_mir(&resolved.module, &checker, &mut sink);

$ python3 scripts/manager.py selfhost diag-parity   # baseline (Rust bin built)
✓ selfhost diagnostic parity   (PASS)
```

No source files in `crates/arukellt/`, `scripts/run/arukellt-selfhost.sh`,
or `docs/current-state.md` were modified by this attempt. Only this
status note is committed.
