# 561 — Phase 5: Delete `crates/ark-mir`

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 561
**Depends on**: 559
**Track**: selfhost-retirement
**Orchestration class**: implementation-ready
**Orchestration upstream**: #559
**Blocks**: 564
**Blocks v5**: no
**Source**: #529 Phase 5 — Core compiler crate (MIR data structures and lowering passes).

**Implementation target**: Per #529 Phase 5, this issue removes exactly one Rust crate (`crates/ark-mir`). No Ark product code is added or changed; this is retirement work scoped to a single crate.

## Summary

`crates/ark-mir` is targeted for deletion in Phase 5 of #529. This issue performs **only** the deletion of that single crate and the immediate workspace / dependency / CI references to it. No other crate is touched.

Only attempt after `lower_hir_to_mir` is fully selfhost-driven and `crates/ark-driver` no longer depends on `crates/ark-mir` for lowering.

## Pre-deletion scan note

Status: blocked-by-upstream. The repository scan still shows live `ark_mir` consumers outside `crates/ark-mir`, so this issue is not ready for deletion yet.

Active consumers from the scan:

- `crates/ark-llvm/Cargo.toml:10` (`ark-mir = { path = "../ark-mir" }`)
- `crates/ark-llvm/src/emit.rs:7` (`use ark_mir::mir::*;`)
- `crates/ark-wasm/Cargo.toml:7` (`ark-mir = { workspace = true }`)
- `crates/ark-wasm/src/component/wit_parse.rs:563`
- `crates/ark-wasm/src/component/mod.rs:22`
- `crates/ark-wasm/src/emit/mod.rs:14`
- `crates/ark-wasm/src/emit/t1/mod.rs:13`
- `crates/ark-wasm/src/emit/t2_freestanding.rs:9`
- `crates/ark-wasm/src/emit/t3/mod.rs:29`
- `crates/ark-wasm/src/emit/t3/helpers.rs:6`
- `crates/ark-wasm/src/emit/t3_wasm_gc/*`

Commands run:

```bash
rg -n "ark[-_]mir|ark_mir" crates/ src/ scripts/ .github/workflows/ docs/ --glob '!issues/done/**'
rg -n "ark_mir|ark-mir" Cargo.toml Cargo.lock crates/*/Cargo.toml
```

## Pre-deletion invariants (must hold before starting)

Record numeric values; do **not** start the deletion if any item is missing.

- [ ] `python scripts/manager.py selfhost fixpoint` rc=0
- [ ] `python scripts/manager.py selfhost fixture-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py selfhost parity --mode --cli` PASS=<N> FAIL=0 (record baseline)
- [ ] `python scripts/manager.py selfhost diag-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py verify` rc=0 (record baseline)
- [ ] No remaining `cargo run -p ark-mir`-style invocation anywhere reachable from `scripts/` or `.github/workflows/` (verified by `rg "ark-mir" scripts/ .github/workflows/`)
- [ ] All consumers of `ark_mir` symbols outside the crate itself have already been migrated to selfhost (`src/`) or to a remaining crate (verified by `rg "ark_mir" crates/ src/ scripts/` showing only the crate itself plus explicitly-allowed comments)

## Acceptance

- [ ] `crates/ark-mir/` directory removed (`[ ! -d crates/ark-mir ]`)
- [ ] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-mir`
- [ ] No other crate's `Cargo.toml` lists `ark-mir` as a `[dependencies]` / `[dev-dependencies]` entry (`grep -RIn "^ark-mir\b\|\"ark-mir\"" crates/*/Cargo.toml` empty)
- [ ] `Cargo.lock` regenerated (run `cargo metadata --format-version 1 --offline 2>/dev/null || cargo check --workspace`) and committed without `name = "ark-mir"`
- [ ] No source / script / docs reference: `rg -l "\bark_mir\b\|\bark-mir\b" crates/ scripts/ src/ docs/ .github/` returns only entries explicitly enumerated in the close note (e.g. archived ADRs)
- [ ] `python scripts/manager.py verify` rc=0
- [ ] 4 canonical selfhost gates: rc=0, no FAIL increase, no SKIP increase

## Required verification (close gate)

Each command MUST be executed; record exit code and (where applicable) PASS/FAIL/SKIP counts in the close note.

```bash
python scripts/manager.py verify
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost parity --mode --cli
python scripts/manager.py selfhost diag-parity
cargo check --workspace
rg -l "\bark_mir\b" crates/ scripts/ src/ docs/ .github/
```

**REBUILD_BEFORE_VERIFY**: yes (workspace topology change forces selfhost rebuild)

## STOP_IF

- Any consumer in another crate / script / workflow still references this crate at deletion time → open a focused migration issue, mark this one `blocked-by-upstream`, **STOP**.
- Removing the crate causes any of the 4 canonical gates to regress (FAIL>0 or SKIP delta > 0) → revert the deletion commit and **STOP**.
- Removing the crate causes any fixture in `tests/fixtures/` to fail → revert and **STOP**.
- `cargo check --workspace` fails after removal → revert and **STOP**.
- A reverse-dependency was missed and surfaces only in CI → revert and **STOP**.

## False-done prevention checklist (close-gate reviewer must verify all)

The reviewer is a **different agent** from the implementer (`verify-issue-closure`). Each line must be checked with command output cited in the close note.

1. [ ] Directory truly absent: `test ! -d crates/ark-mir` exit 0
2. [ ] No workspace member ref: `grep -F "crates/ark-mir" Cargo.toml` empty
3. [ ] No reverse dep ref: `grep -RIn "\bark-mir\b" crates/*/Cargo.toml` empty
4. [ ] No Rust source ref: `rg -l "\bark_mir\b" crates/ src/` empty
5. [ ] No script / CI ref: `rg -l "\bark-mir\b" scripts/ .github/workflows/` empty
6. [ ] No docs ref: `rg -l "\bark_mir\b\|\bark-mir\b" docs/` returns only paths listed in the close note (archived ADRs allowed if explicitly enumerated)
7. [ ] All 4 canonical gates: numeric Δ recorded showing `FAIL=0` and `SKIP_delta=0`
8. [ ] `cargo check --workspace` rc=0 (output excerpt cited)
9. [ ] commit hash listed; `git show --stat <hash>` shows only files within PRIMARY / ALLOWED ADJACENT paths
10. [ ] `python scripts/check/check-docs-consistency.py` rc=0 if docs were touched

## Primary paths

- `crates/ark-mir/` (deletion)
- `Cargo.toml` (workspace `members`)
- `Cargo.lock` (regeneration)

## Allowed adjacent paths

- `Cargo.toml` of OTHER crates: **only** to remove a `[dependencies]` / `[dev-dependencies]` entry on `ark-mir`
- `.github/workflows/*.yml`: **only** to remove direct invocations of this crate
- `docs/current-state.md`: to reflect the deletion (single-line edit)
- `docs/adr/`: only if a new ADR is required to record the retirement

## Forbidden paths

- `src/compiler/*.ark` (no Ark product changes in this slice)
- Any other `crates/` directory beyond the dependency-removal allowance above
- `scripts/selfhost/checks.py` `FIXTURE_PARITY_SKIP` / `DIAG_PARITY_SKIP` (no SKIP additions ever)
- `tests/fixtures/**` (no fixture additions / deletions)

## Commit discipline

- Single logical commit.
- Suggested message: `chore(crates): remove crates/ark-mir per #529 Phase 5 (closes #561)`

## Close-note evidence schema (required)

```text
commit: <hash>
gates (baseline → post):
  fixpoint:        rc=0 → rc=0
  fixture parity:  PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
  cli parity:      PASS=<N> FAIL=0       → PASS=<N> FAIL=0
  diag parity:     PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
cargo check --workspace: rc=0
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓ 10✓
remaining references (if any): <list with justification>
```
