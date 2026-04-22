# 585 — Replace Rust-baseline parity gates with selfhost-native verification contract

**Status**: open
**Track**: selfhost-retirement
**Depends on**: 559
**Blocks**: 583, 560, 561, 562, 563, 564
**Orchestration class**: verification-contract

## Why

#583 (retire `ARUKELLT_USE_RUST=1` and purge `arukellt` Rust core consumers)
STOPped at slice-attempt because `scripts/selfhost/checks.py` hard-requires
the Rust `target/debug/arukellt` binary as the parity baseline for all 4
canonical selfhost gates:

1. `_find_arukellt()` is a hard precondition for `run_fixpoint`,
   `run_fixture_parity`, `run_diag_parity`, `_run_cli_parity`.
2. `run_fixpoint` Stage 1 invokes the Rust binary to compile
   `src/compiler/main.ark` to a Stage-1 wasm — `.build/` and
   `.bootstrap-build/` are gitignored, so no committed bootstrap wasm
   exists. On a fresh clone, only the Rust binary can bootstrap.
3. `run_diag_parity` enforces `pass_count >= 10` AND requires the Rust
   binary's `check fixture.ark` output to literally contain each `.diag`
   pattern. A stubbed/missing Rust binary classifies every fixture as
   "skip (Rust: pattern not found)" → 0 pass → FAIL.
4. `_run_cli_parity` byte-compares Rust vs selfhost `--version`/`--help`.

The whole Phase 5 deletion chain (#560/#561/#562/#563/#564) is therefore
gated on retiring or replacing the Rust-baseline parity contract.

This issue redesigns the verification contract so the gates can survive
without the Rust binary, unblocking #583 and Phase 5.

## Pre-condition invariants

1. 4 canonical selfhost gates currently PASS at HEAD (with Rust binary
   present).
2. `scripts/run/arukellt-selfhost.sh` (selfhost-first wrapper) is the
   default user-facing entry per #559.

## Acceptance

- [ ] An ADR under `docs/adr/` records the new verification contract:
  what replaces "selfhost-vs-Rust" parity, how bootstrap works on a
  fresh clone, and what guarantees each gate now provides.
- [ ] `scripts/selfhost/checks.py` is updated to the new contract:
  - `run_fixpoint` no longer requires the Rust binary; bootstrap uses a
    committed pinned-reference wasm (or equivalent mechanism documented
    in the ADR).
  - `run_fixture_parity` is replaced with selfhost-only fixture coverage
    OR pinned-reference-wasm-vs-current-selfhost comparison.
  - `run_diag_parity` is replaced with a pure selfhost diagnostic
    snapshot test (e.g., golden `.diag` files compared against current
    selfhost output).
  - `_run_cli_parity` is replaced with a pure selfhost `--version` /
    `--help` snapshot test.
  - The total PASS/FAIL/SKIP counts at the new contract's baseline are
    recorded in the ADR.
- [ ] A pinned-reference selfhost wasm artifact (or equivalent) is
  committed under a tracked path (e.g. `.bootstrap/` removed from
  gitignore for that one file, OR a new `bootstrap/` directory).
  The artifact's provenance and refresh cadence are documented.
- [ ] All 4 reframed gates PASS without a Rust binary in `target/`.
- [ ] On a fresh clone (`git clean -dfx && cargo clean`) the 4 gates
  bootstrap and PASS using only the committed selfhost artifact.
- [ ] Documentation references updated:
  `docs/current-state.md`, `docs/process/selfhost-bootstrap.md` (if it
  exists), `README.md` if relevant.

## Required verification

1. `python3 scripts/manager.py selfhost fixpoint`
2. `python3 scripts/manager.py selfhost fixture-parity`
3. `python3 scripts/manager.py selfhost parity --mode --cli`
4. `python3 scripts/manager.py selfhost diag-parity`
5. Fresh-clone simulation: in a clean directory (or with
   `target/debug/arukellt` removed and `cargo clean` run), all 4 gates
   PASS using only committed artifacts.
6. `cargo build --workspace --exclude ark-llvm` still succeeds
   (Rust crates not yet deleted; this slice only changes the gate
   contract).

## STOP_IF

- The reframed contract weakens behavioral coverage in a way the ADR
  cannot justify (e.g. eliminates diag coverage entirely).
- A committed pinned-reference wasm cannot be reproducibly built —
  document the reproducibility gap and stop.

## False-done prevention checklist

- Do NOT make the gates trivially pass by removing all assertions.
- The new diag-parity replacement must cover at least the same fixture
  count (`pass_count >= 10` floor preserved or improved).
- The new fixture-parity replacement must demonstrate behavioral
  coverage (selfhost runs all fixtures and outputs match a tracked
  golden, not just "selfhost ran without crashing").
- Do NOT delete `crates/arukellt` or any Rust core crate as part of
  this slice — that's #583/#560/#561/#562/#563/#564 follow-on.
- Do NOT add SKIPs to the new contract to make it pass.

## PRIMARY paths

- `docs/adr/NNNN-selfhost-native-verification-contract.md` (NEW)
- `scripts/selfhost/checks.py` (rewrite of the 4 gate functions)
- `scripts/manager.py` (only if its CLI surface changes)
- `bootstrap/` or equivalent (NEW directory for pinned reference wasm)
- `.gitignore` (carve-out for the pinned reference artifact)
- `docs/current-state.md`
- `docs/process/selfhost-bootstrap.md` (if exists)

## ALLOWED paths (read / minor edit)

- `scripts/selfhost/` helpers
- `tests/fixtures/selfhost*/` for fixture inventory
- `Makefile` / `mise.toml` if their `verify` targets reference Rust
  binary explicitly

## FORBIDDEN paths

- `crates/**` (no Rust source edits in this slice)
- `src/compiler/*.ark` (no selfhost source edits)
- `tests/fixtures/**/*.ark` (no fixture content changes; you may add
  new fixtures only if the ADR justifies them)
- Any other open issue file

## Close-note evidence schema

- ADR file path
- Pinned-reference wasm artifact path + size + sha256
- Diff stats for `scripts/selfhost/checks.py`
- 4 gate logs at HEAD (with `target/debug/arukellt` deleted)
- Fresh-clone simulation log
- Deferred items / follow-on issues
