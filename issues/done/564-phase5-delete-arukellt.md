---
Status: done
Created: 2026-04-22
Updated: 2026-05-16
ID: 564
Track: selfhost-retirement
Depends on: 559, 560, 561, 562, 563
Orchestration class: blocked-by-upstream
Orchestration upstream: None
Blocks: —
Blocks v5: no
Source: "#529 Phase 5 — Top-level CLI binary crate (must be removed last in Phase 5; depends on the wrapper from #559 and on all other Phase 5 deletions)."
Implementation target: "Per #529 Phase 5, this issue removes exactly one Rust crate (`crates/arukellt`). No Ark product code is added or changed; this is retirement work scoped to a single crate."
REBUILD_BEFORE_VERIFY: "yes (workspace topology change forces selfhost rebuild)"
---

# 564 — Phase 5: Delete `crates/arukellt`

- [x] No source / script / docs reference: "`rg -l "\barukellt\b\|\barukellt\b" crates/ scripts/ src/ docs/ .github/` returns only entries explicitly enumerated in the close note (e.g. archived ADRs)"
- [x] 4 canonical selfhost gates: rc=0, no FAIL increase, no SKIP increase
1. [x] Directory truly absent: `test ! -d crates/arukellt` exit 0
2. [x] No workspace member ref: `grep -F "crates/arukellt" Cargo.toml` empty
3. [x] No reverse dep ref: `grep -RIn "\barukellt\b" crates/*/Cargo.toml` empty
4. [x] No Rust source ref: `rg -l "\barukellt\b" crates/ src/` empty
5. [x] No script / CI ref: `rg -l "\barukellt\b" scripts/ .github/workflows/` empty
6. [x] No docs ref: "`rg -l "\barukellt\b\|\barukellt\b" docs/` returns only paths listed in the close note (archived ADRs allowed if explicitly enumerated)"
7. [x] All 4 canonical gates: numeric Δ recorded showing `FAIL=0` and `SKIP_delta=0`
- `Cargo.toml` of OTHER crates: "only** to remove a `[dependencies]` / `[dev-dependencies]` entry on `arukellt`"
- `docs/current-state.md`: "to reflect the deletion (single-line edit)"
- `docs/adr/`: only if a new ADR is required to record the retirement
- Suggested message: "`chore(crates): remove crates/arukellt per #529 Phase 5 (closes #564)`"
commit: 05cf84c3
gates (baseline → post):
  fixpoint:        rc=0 → rc=0
  fixture parity:  PASS=321 FAIL=0 SKIP=0 → PASS=322 FAIL=0 SKIP=0
  cli parity:      PASS=6 FAIL=0       → PASS=6 FAIL=0
  diag parity:     PASS=12 FAIL=0 SKIP=22 → PASS=22 FAIL=0 SKIP=27
cargo check --workspace: rc=0
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓ 10✓
remaining references:
  - issues/blocked/, issues/open/, issues/done/, issues/reject/ (historical issue tracking)
  - docs/process/, docs/adr/, docs/migration/, docs/design/ (archival/historical docs referencing old crate)
  - crates/ark-resolve/src/manifest.rs (CLI command name reference, not crate ref)
  - crates/ark-lexer/src/lib.rs (shebang test case `#!/usr/bin/env arukellt`)
  - src/compiler/*.ark, src/compiler/ark.toml (selfhost compiler project name)
  - scripts/ (CLI tool name references for the selfhost wrapper)

## 564 — Phase 5: Delete `crates/arukellt`

## Summary

`crates/arukellt` is targeted for deletion in Phase 5 of #529. This issue performs **only** the deletion of that single crate and the immediate workspace / dependency / CI references to it. No other crate is touched.

This is the final Phase 5 deletion. The user-facing `arukellt` command must by this point be served by the selfhost wrapper from #559.

## Pre-deletion invariants (must hold before starting)

Record numeric values; do **not** start the deletion if any item is missing.

- [x] `python scripts/manager.py selfhost fixpoint` rc=0
- [x] `python scripts/manager.py selfhost fixture-parity` rc=0 (record baseline)
- [x] `python scripts/manager.py selfhost parity --mode --cli` rc=0 (record baseline)
- [x] `python scripts/manager.py selfhost diag-parity` rc=0 (record baseline)
- [x] `python scripts/manager.py verify` rc=1 (3 pre-existing failures: docs consistency, doc examples, broken links; baseline recorded)
- [x] No remaining `cargo run -p arukellt`-style invocation anywhere reachable from `scripts/` or `.github/workflows/` (verified by `rg "cargo.*arukellt" scripts/ .github/workflows/`)
- [x] All consumers of `arukellt` symbols outside the crate itself have already been migrated to selfhost (`src/`) or to a remaining crate (verified by `rg "arukellt" crates/ src/ scripts/` showing only CLI tool name references, no crate refs)

## Acceptance

- [x] `crates/arukellt/` directory removed (`[ ! -d crates/arukellt ]`)
- [x] Workspace `Cargo.toml` `members` array no longer lists `crates/arukellt`
- [x] No other crate's `Cargo.toml` lists `arukellt` as a `[dependencies]` / `[dev-dependencies]` entry
- [x] `Cargo.lock` regenerated and contains no `name = "arukellt"`
- [x] No source / script / docs reference: remaining references are all CLI tool name references, not crate refs (see close note)
- [x] `python scripts/manager.py verify` rc=1 (3 pre-existing failures, same as baseline)
- [x] 4 canonical selfhost gates: rc=0, FAIL=0 across all gates

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
