# 572 — Phase 7: Delete `crates/ark-lsp`

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 572
**Depends on**: 569, 570
**Track**: selfhost-retirement
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #569, #570
**Blocks**: 582
**Blocks v5**: no
**Source**: #529 Phase 7 — Rust LSP server crate (replaced by `src/ide/lsp.ark`).

**Implementation target**: Per #529 Phase 7, this issue removes exactly one Rust crate (`crates/ark-lsp`). No Ark product code is added or changed; this is retirement work scoped to a single crate.

## Summary

`crates/ark-lsp` is targeted for deletion in Phase 7 of #529. This issue performs **only** the deletion of that single crate and the immediate workspace / dependency / CI references to it. No other crate is touched.

Only attempt after the VS Code extension and any other LSP consumer has been switched to the Ark `src/ide/lsp.ark` server (verified by extension manifest / launch config inspection).

## Pre-deletion invariants (must hold before starting)

Record numeric values; do **not** start the deletion if any item is missing.

- [ ] `python scripts/manager.py selfhost fixpoint` rc=0
- [ ] `python scripts/manager.py selfhost fixture-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py selfhost parity --mode --cli` PASS=<N> FAIL=0 (record baseline)
- [ ] `python scripts/manager.py selfhost diag-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py verify` rc=0 (record baseline)
- [ ] No remaining `cargo run -p ark-lsp`-style invocation anywhere reachable from `scripts/` or `.github/workflows/` (verified by `rg "ark-lsp" scripts/ .github/workflows/`)
- [ ] All consumers of `ark_lsp` symbols outside the crate itself have already been migrated to selfhost (`src/`) or to a remaining crate (verified by `rg "ark_lsp" crates/ src/ scripts/` showing only the crate itself plus explicitly-allowed comments)

## Acceptance

- [ ] `crates/ark-lsp/` directory removed (`[ ! -d crates/ark-lsp ]`)
- [ ] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-lsp`
- [ ] No other crate's `Cargo.toml` lists `ark-lsp` as a `[dependencies]` / `[dev-dependencies]` entry (`grep -RIn "^ark-lsp\b\|\"ark-lsp\"" crates/*/Cargo.toml` empty)
- [ ] `Cargo.lock` regenerated (run `cargo metadata --format-version 1 --offline 2>/dev/null || cargo check --workspace`) and committed without `name = "ark-lsp"`
- [ ] No source / script / docs reference: `rg -l "\bark_lsp\b\|\bark-lsp\b" crates/ scripts/ src/ docs/ .github/` returns only entries explicitly enumerated in the close note (e.g. archived ADRs)
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
rg -l "\bark_lsp\b" crates/ scripts/ src/ docs/ .github/
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

1. [ ] Directory truly absent: `test ! -d crates/ark-lsp` exit 0
2. [ ] No workspace member ref: `grep -F "crates/ark-lsp" Cargo.toml` empty
3. [ ] No reverse dep ref: `grep -RIn "\bark-lsp\b" crates/*/Cargo.toml` empty
4. [ ] No Rust source ref: `rg -l "\bark_lsp\b" crates/ src/` empty
5. [ ] No script / CI ref: `rg -l "\bark-lsp\b" scripts/ .github/workflows/` empty
6. [ ] No docs ref: `rg -l "\bark_lsp\b\|\bark-lsp\b" docs/` returns only paths listed in the close note (archived ADRs allowed if explicitly enumerated)
7. [ ] All 4 canonical gates: numeric Δ recorded showing `FAIL=0` and `SKIP_delta=0`
8. [ ] `cargo check --workspace` rc=0 (output excerpt cited)
9. [ ] commit hash listed; `git show --stat <hash>` shows only files within PRIMARY / ALLOWED ADJACENT paths
10. [ ] `python scripts/check/check-docs-consistency.py` rc=0 if docs were touched

## Primary paths

- `crates/ark-lsp/` (deletion)
- `Cargo.toml` (workspace `members`)
- `Cargo.lock` (regeneration)

## Allowed adjacent paths

- `Cargo.toml` of OTHER crates: **only** to remove a `[dependencies]` / `[dev-dependencies]` entry on `ark-lsp`
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
- Suggested message: `chore(crates): remove crates/ark-lsp per #529 Phase 7 (closes #572)`

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
