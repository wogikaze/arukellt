---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 582
Track: selfhost-retirement
Depends on: 572, 573, 574, 575, 576, 577, 578, 579, 580, 581
Orchestration class: blocked-by-upstream
Orchestration upstream: None
Blocks: —
Blocks v5: no
Source: "#529 Phase 7 — Full Rust Deletion (final step)"
Implementation target: "Final step of #529. After every remaining Rust crate has been deleted (issues #572–#581 closed), this issue removes the workspace `Cargo.toml`, the top-level `Cargo.lock`, and any leftover Rust-only tooling reference. After this commit, the repository must contain no `crates/` directory and no `Cargo.{toml,lock}` at the workspace root."
REBUILD_BEFORE_VERIFY: "yes (entire build system change)"
CI run URL: <url>
---

# 582 — Phase 7 final: remove `Cargo.toml` and `Cargo.lock`
4. [ ] All 4 canonical gates green: numeric Δ recorded
- Single logical commit (the symbolic "delete Rust" commit). Suggested message: "`chore(repo): remove Rust workspace per #529 Phase 7 final (closes #582)`."
commit: <hash>
fixpoint: rc=0 → rc=0
fixture parity: PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
cli parity: PASS=<N> FAIL=0       → PASS=<N> FAIL=0
diag parity: PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
remaining `cargo` references: <list with per-line justification>
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓
# 582 — Phase 7 final: remove `Cargo.toml` and `Cargo.lock`


## Summary

Last step of #529 Phase 7. Once every remaining Rust crate has been deleted (issues #572–#581 closed), the workspace `Cargo.toml`, the top-level `Cargo.lock`, and any leftover Rust-only tooling reference must be removed. After this commit, the repository must contain no `crates/` directory and no `Cargo.{toml,lock}`.

## Pre-deletion invariants (must hold before starting)

- [ ] All upstream issues #572–#581 are in `issues/done/`
- [ ] `[ ! -d crates ]` (or `crates/` exists only as an empty directory to be removed in this commit)
- [ ] `python scripts/manager.py verify` rc=0
- [ ] All 4 canonical selfhost gates rc=0 with FAIL=0 and SKIP delta = 0
- [ ] No `cargo` invocation remains anywhere reachable from `scripts/` or `.github/workflows/` (`rg -n "\bcargo\b" scripts/ .github/workflows/` returns only explicitly enumerated entries)

## Acceptance

- [ ] Workspace `Cargo.toml` removed
- [ ] Top-level `Cargo.lock` removed
- [ ] `crates/` directory removed (if not already)
- [ ] `rust-toolchain.toml` / `rust-toolchain` / `.cargo/` removed if present and unused
- [ ] No `cargo` reference remains in `scripts/`, `.github/workflows/`, `docs/`, or `mise.toml` (`rg -n "\bcargo\b" scripts/ .github/workflows/ docs/ mise.toml` returns only explicitly enumerated archived entries)
- [ ] `docs/current-state.md` reflects the Rust-free architecture
- [ ] `README.md` no longer documents Rust build prerequisites
- [ ] `python scripts/manager.py verify` rc=0 with no Rust toolchain installed in CI
- [ ] 4 canonical selfhost gates rc=0 with FAIL=0 and SKIP delta = 0

## Required verification (close gate)

```bash
python scripts/manager.py verify
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost parity --mode --cli
python scripts/manager.py selfhost diag-parity
rg -n "\bcargo\b" scripts/ .github/workflows/ docs/ mise.toml
test ! -e Cargo.toml
test ! -e Cargo.lock
test ! -d crates
python scripts/check/check-docs-consistency.py
```


## STOP_IF

- Any upstream issue (#572–#581) is still open → mark `blocked-by-upstream`, **STOP**.
- Any `cargo` reference remains that cannot be removed in the same commit → carve out a focused cleanup issue, **STOP**.
- Any of the 4 canonical gates regresses after the removal → revert and **STOP**.
- CI fails because the runner image lacks selfhost prerequisites → fix runner image and **STOP** until CI is green.

## False-done prevention checklist (close-gate reviewer)

1. [ ] `test ! -e Cargo.toml && test ! -e Cargo.lock && test ! -d crates` exit 0
2. [ ] `rg -n "\bcargo\b" scripts/ .github/workflows/ docs/ mise.toml` output cited; remaining entries justified per-line
3. [ ] All upstream issues (#572–#581) confirmed in `issues/done/` (cite filenames)
4. [ ] All 4 canonical gates green: numeric Δ recorded
5. [ ] `python scripts/manager.py verify` rc=0 (output excerpt cited)
6. [ ] CI green on the commit (cite workflow run URL)
7. [ ] `docs/current-state.md` and `README.md` diffs cited
8. [ ] `python scripts/check/check-docs-consistency.py` rc=0
9. [ ] commit hash listed; `git show --stat <hash>` shows only PRIMARY / ALLOWED ADJACENT paths

## Primary paths

- `Cargo.toml` (deletion)
- `Cargo.lock` (deletion)
- `crates/` (deletion if non-empty)
- `rust-toolchain.toml` / `rust-toolchain` / `.cargo/` (deletion if present)

## Allowed adjacent paths

- `scripts/manager.py`, `scripts/run/*`, `scripts/check/*`, `scripts/selfhost/*` (only to remove `cargo` invocations)
- `.github/workflows/*.yml` (only to remove `cargo`/Rust setup steps)
- `docs/current-state.md`, `README.md`, `docs/adr/` (closeout note)
- `mise.toml` (remove Rust toolchain pin if no longer required)

## Forbidden paths

- `src/` (no Ark product changes in this commit)
- `tests/fixtures/**` (no fixture changes)
- `scripts/selfhost/checks.py` `*_SKIP` lists (no SKIP additions ever)

## Commit discipline

- Single logical commit (the symbolic "delete Rust" commit). Suggested message: `chore(repo): remove Rust workspace per #529 Phase 7 final (closes #582)`.

## Close-note evidence schema (required)

```text
commit: <hash>
upstream issues closed (cite paths):
  issues/done/572-...
  issues/done/573-...
  ...
  issues/done/581-...
gates (baseline → post):
  fixpoint:        rc=0 → rc=0
  fixture parity:  PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
  cli parity:      PASS=<N> FAIL=0       → PASS=<N> FAIL=0
  diag parity:     PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
remaining `cargo` references: <list with per-line justification>
CI run URL: <url>
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓
```