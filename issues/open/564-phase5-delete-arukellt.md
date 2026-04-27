---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 564
Track: selfhost-retirement
Depends on: 559, 560, 561, 562, 563
Orchestration class: blocked-by-upstream
Orchestration upstream: #559, #560, #561, #562, #563
---

# 564 — Phase 5: Delete `crates/arukellt`
**Blocks**: —
**Blocks v5**: no
**Source**: #529 Phase 5 — Top-level CLI binary crate (must be removed last in Phase 5; depends on the wrapper from #559 and on all other Phase 5 deletions).

**Implementation target**: Per #529 Phase 5, this issue removes exactly one Rust crate (`crates/arukellt`). No Ark product code is added or changed; this is retirement work scoped to a single crate.

## Summary

`crates/arukellt` is targeted for deletion in Phase 5 of #529. This issue performs **only** the deletion of that single crate and the immediate workspace / dependency / CI references to it. No other crate is touched.

This is the final Phase 5 deletion. The user-facing `arukellt` command must by this point be served by the selfhost wrapper from #559.

## Pre-deletion invariants (must hold before starting)

Record numeric values; do **not** start the deletion if any item is missing.

- [ ] `python scripts/manager.py selfhost fixpoint` rc=0
- [ ] `python scripts/manager.py selfhost fixture-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py selfhost parity --mode --cli` PASS=<N> FAIL=0 (record baseline)
- [ ] `python scripts/manager.py selfhost diag-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py verify` rc=0 (record baseline)
- [ ] No remaining `cargo run -p arukellt`-style invocation anywhere reachable from `scripts/` or `.github/workflows/` (verified by `rg "arukellt" scripts/ .github/workflows/`)
- [ ] All consumers of `arukellt` symbols outside the crate itself have already been migrated to selfhost (`src/`) or to a remaining crate (verified by `rg "arukellt" crates/ src/ scripts/` showing only the crate itself plus explicitly-allowed comments)

## Acceptance

- [ ] `crates/arukellt/` directory removed (`[ ! -d crates/arukellt ]`)
- [ ] Workspace `Cargo.toml` `members` array no longer lists `crates/arukellt`
- [ ] No other crate's `Cargo.toml` lists `arukellt` as a `[dependencies]` / `[dev-dependencies]` entry (`grep -RIn "^arukellt\b\|\"arukellt\"" crates/*/Cargo.toml` empty)
- [ ] `Cargo.lock` regenerated (run `cargo metadata --format-version 1 --offline 2>/dev/null || cargo check --workspace`) and committed without `name = "arukellt"`
- [ ] No source / script / docs reference: `rg -l "\barukellt\b\|\barukellt\b" crates/ scripts/ src/ docs/ .github/` returns only entries explicitly enumerated in the close note (e.g. archived ADRs)
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
rg -l "\barukellt\b" crates/ scripts/ src/ docs/ .github/
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

1. [ ] Directory truly absent: `test ! -d crates/arukellt` exit 0
2. [ ] No workspace member ref: `grep -F "crates/arukellt" Cargo.toml` empty
3. [ ] No reverse dep ref: `grep -RIn "\barukellt\b" crates/*/Cargo.toml` empty
4. [ ] No Rust source ref: `rg -l "\barukellt\b" crates/ src/` empty
5. [ ] No script / CI ref: `rg -l "\barukellt\b" scripts/ .github/workflows/` empty
6. [ ] No docs ref: `rg -l "\barukellt\b\|\barukellt\b" docs/` returns only paths listed in the close note (archived ADRs allowed if explicitly enumerated)
7. [ ] All 4 canonical gates: numeric Δ recorded showing `FAIL=0` and `SKIP_delta=0`
8. [ ] `cargo check --workspace` rc=0 (output excerpt cited)
9. [ ] commit hash listed; `git show --stat <hash>` shows only files within PRIMARY / ALLOWED ADJACENT paths
10. [ ] `python scripts/check/check-docs-consistency.py` rc=0 if docs were touched

## Primary paths

- `crates/arukellt/` (deletion)
- `Cargo.toml` (workspace `members`)
- `Cargo.lock` (regeneration)

## Allowed adjacent paths

- `Cargo.toml` of OTHER crates: **only** to remove a `[dependencies]` / `[dev-dependencies]` entry on `arukellt`
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
- Suggested message: `chore(crates): remove crates/arukellt per #529 Phase 5 (closes #564)`

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
