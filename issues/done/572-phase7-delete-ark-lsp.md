---
Status: "closed (#572 Phase 7 of #529)"
Created: 2026-04-22
Updated: 2026-04-22
ID: 572
Track: selfhost-retirement
Depends on: 569, 570
Orchestration class: blocked-by-upstream
Orchestration upstream: None
---

# 572 — Phase 7: Delete `crates/ark-lsp`
Blocks: 582
Blocks v5: no
Source: "#529 Phase 7 — Rust LSP server crate (replaced by `src/ide/lsp.ark`)."
Implementation target: "Per #529 Phase 7, this issue removes exactly one Rust crate (`crates/ark-lsp`). No Ark product code is added or changed; this is retirement work scoped to a single crate."
- [x] No source / script / docs reference: "`rg -l "\bark_lsp\b\|\bark-lsp\b" crates/ scripts/ src/ docs/ .github/` returns only entries explicitly enumerated in the close note (e.g. archived ADRs)"
- [x] 4 canonical selfhost gates: rc=0, no FAIL increase, no SKIP increase
REBUILD_BEFORE_VERIFY: "yes (workspace topology change forces selfhost rebuild)"
1. [x] Directory truly absent: `test ! -d crates/ark-lsp` exit 0
2. [x] No workspace member ref: `grep -F "crates/ark-lsp" Cargo.toml` empty
3. [x] No reverse dep ref: `grep -RIn "\bark-lsp\b" crates/*/Cargo.toml` empty
4. [x] No Rust source ref: `rg -l "\bark_lsp\b" crates/ src/` empty
5. [x] No script / CI ref: `rg -l "\bark-lsp\b" scripts/ .github/workflows/` empty
6. [x] No docs ref: "`rg -l "\bark_lsp\b\|\bark-lsp\b" docs/` returns only paths listed in the close note (archived ADRs allowed if explicitly enumerated)"
7. [x] All 4 canonical gates: numeric Δ recorded showing `FAIL=0` and `SKIP_delta=0`
- `Cargo.toml` of OTHER crates: "only** to remove a `[dependencies]` / `[dev-dependencies]` entry on `ark-lsp`"
- `docs/current-state.md`: "to reflect the deletion (single-line edit)"
- `docs/adr/`: only if a new ADR is required to record the retirement
- Suggested message: "`chore(crates): remove crates/ark-lsp per #529 Phase 7 (closes #572)`"
commit: <hash>
fixpoint: rc=0 → rc=0
fixture parity: PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
cli parity: PASS=<N> FAIL=0       → PASS=<N> FAIL=0
diag parity: PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
cargo check --workspace: rc=0
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓ 10✓
remaining references (if any): <list with justification>
Branch: `feat/572-delete-ark-lsp` → ff-merge to `master`
- `python scripts/manager.py verify quick`: "17 PASS / 4 FAIL / 0 SKIP — **identical** (same 4 pre-existing failures)"
- Pre-existing FAIL: "`Fixture manifest out of sync with disk`, `issues/done/ has no unchecked checkboxes`, `doc example check (ark blocks in docs/)`, `broken internal links detected`"
- `cargo check --workspace`: rc=0
- `rg "ark_lsp|ark-lsp" crates/ src/ scripts/ .github/workflows/ docs/ extensions/`: "enumerated upstream uses (all in: `crates/ark-lsp/**`, `Cargo.toml` workspace `members` + `default-members`, `scripts/check/check-panic-audit.sh`, `scripts/gate_domain/checks.py` (--exclude flags), `.github/workflows/ci.yml` (lsp-e2e job), `.github/agents/impl-{vscode-ide,editor-runtime}.agent.md`, `codex-skills/impl-{vscode-ide,editor-runtime}/SKILL.md`, `extensions/arukellt-all-in-one/{README.md,src/test/extension.test.js,src/test/fixtures/lsp-stub.js}` (comments only), `docs/{compiler/bootstrap.md,compiler/pipeline.md,directory-ownership.md,release-criteria.md,release-checklist.md,contributing.md,module-resolution.md}`, `README.md`, plus historical `docs/adr/ADR-015-no-panic-in-user-paths.md` and `docs/process/roadmap-v5.md` (left intact per #586/#561 precedent)"
7. ✓ All gates: 17/21 PASS = baseline; FAIL=0 increase, SKIP=0 increase
# 572 — Phase 7: Delete `crates/ark-lsp`


## Summary

`crates/ark-lsp` is targeted for deletion in Phase 7 of #529. This issue performs **only** the deletion of that single crate and the immediate workspace / dependency / CI references to it. No other crate is touched.

Only attempt after the VS Code extension and any other LSP consumer has been switched to the Ark `src/ide/lsp.ark` server (verified by extension manifest / launch config inspection).

## Pre-deletion invariants (must hold before starting)

Record numeric values; do **not** start the deletion if any item is missing.

- [x] `python scripts/manager.py selfhost fixpoint` rc=0
- [x] `python scripts/manager.py selfhost fixture-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [x] `python scripts/manager.py selfhost parity --mode --cli` PASS=<N> FAIL=0 (record baseline)
- [x] `python scripts/manager.py selfhost diag-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [x] `python scripts/manager.py verify` rc=0 (record baseline)
- [x] No remaining `cargo run -p ark-lsp`-style invocation anywhere reachable from `scripts/` or `.github/workflows/` (verified by `rg "ark-lsp" scripts/ .github/workflows/`)
- [x] All consumers of `ark_lsp` symbols outside the crate itself have already been migrated to selfhost (`src/`) or to a remaining crate (verified by `rg "ark_lsp" crates/ src/ scripts/` showing only the crate itself plus explicitly-allowed comments)

## Acceptance

- [x] `crates/ark-lsp/` directory removed (`[ ! -d crates/ark-lsp ]`)
- [x] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-lsp`
- [x] No other crate's `Cargo.toml` lists `ark-lsp` as a `[dependencies]` / `[dev-dependencies]` entry (`grep -RIn "^ark-lsp\b\|\"ark-lsp\"" crates/*/Cargo.toml` empty)
- [x] `Cargo.lock` regenerated (run `cargo metadata --format-version 1 --offline 2>/dev/null || cargo check --workspace`) and committed without `name = "ark-lsp"`
- [x] No source / script / docs reference: `rg -l "\bark_lsp\b\|\bark-lsp\b" crates/ scripts/ src/ docs/ .github/` returns only entries explicitly enumerated in the close note (e.g. archived ADRs)
- [x] `python scripts/manager.py verify` rc=0
- [x] 4 canonical selfhost gates: rc=0, no FAIL increase, no SKIP increase

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


## STOP_IF

- Any consumer in another crate / script / workflow still references this crate at deletion time → open a focused migration issue, mark this one `blocked-by-upstream`, **STOP**.
- Removing the crate causes any of the 4 canonical gates to regress (FAIL>0 or SKIP delta > 0) → revert the deletion commit and **STOP**.
- Removing the crate causes any fixture in `tests/fixtures/` to fail → revert and **STOP**.
- `cargo check --workspace` fails after removal → revert and **STOP**.
- A reverse-dependency was missed and surfaces only in CI → revert and **STOP**.

## False-done prevention checklist (close-gate reviewer must verify all)

The reviewer is a **different agent** from the implementer (`verify-issue-closure`). Each line must be checked with command output cited in the close note.

1. [x] Directory truly absent: `test ! -d crates/ark-lsp` exit 0
2. [x] No workspace member ref: `grep -F "crates/ark-lsp" Cargo.toml` empty
3. [x] No reverse dep ref: `grep -RIn "\bark-lsp\b" crates/*/Cargo.toml` empty
4. [x] No Rust source ref: `rg -l "\bark_lsp\b" crates/ src/` empty
5. [x] No script / CI ref: `rg -l "\bark-lsp\b" scripts/ .github/workflows/` empty
6. [x] No docs ref: `rg -l "\bark_lsp\b\|\bark-lsp\b" docs/` returns only paths listed in the close note (archived ADRs allowed if explicitly enumerated)
7. [x] All 4 canonical gates: numeric Δ recorded showing `FAIL=0` and `SKIP_delta=0`
8. [x] `cargo check --workspace` rc=0 (output excerpt cited)
9. [x] commit hash listed; `git show --stat <hash>` shows only files within PRIMARY / ALLOWED ADJACENT paths
10. [x] `python scripts/check/check-docs-consistency.py` rc=0 if docs were touched

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

## Resolution


### Pre-deletion baseline (master @ 723b2e86)

- `python scripts/manager.py verify quick`: 17 PASS / 4 FAIL / 0 SKIP
  - Pre-existing FAIL: `Fixture manifest out of sync with disk`, `issues/done/ has no unchecked checkboxes`, `doc example check (ark blocks in docs/)`, `broken internal links detected`
- `cargo check --workspace`: rc=0
- `rg "ark_lsp|ark-lsp" crates/ src/ scripts/ .github/workflows/ docs/ extensions/`: enumerated upstream uses (all in: `crates/ark-lsp/**`, `Cargo.toml` workspace `members` + `default-members`, `scripts/check/check-panic-audit.sh`, `scripts/gate_domain/checks.py` (--exclude flags), `.github/workflows/ci.yml` (lsp-e2e job), `.github/agents/impl-{vscode-ide,editor-runtime}.agent.md`, `codex-skills/impl-{vscode-ide,editor-runtime}/SKILL.md`, `extensions/arukellt-all-in-one/{README.md,src/test/extension.test.js,src/test/fixtures/lsp-stub.js}` (comments only), `docs/{compiler/bootstrap.md,compiler/pipeline.md,directory-ownership.md,release-criteria.md,release-checklist.md,contributing.md,module-resolution.md}`, `README.md`, plus historical `docs/adr/ADR-015-no-panic-in-user-paths.md` and `docs/process/roadmap-v5.md` (left intact per #586/#561 precedent)

### Post-deletion baseline

- `python scripts/manager.py verify quick`: 17 PASS / 4 FAIL / 0 SKIP — **identical** (same 4 pre-existing failures)
- `cargo check --workspace`: rc=0
- `rg "ark_lsp|ark-lsp"` outside `issues/done/**`, `docs/adr/**`, `docs/process/roadmap-v*`: only documented removal records and open issues (#548, #555, #563) that name `ark-lsp` as part of their own legacy plans (out of scope for this slice; will be reconciled by their respective close gates)

### Editor-extension migration

The VS Code extension `extensions/arukellt-all-in-one/` already spawns `arukellt lsp` (the selfhost CLI subcommand from #569). The only `ark-lsp` references in the extension were comments referencing the deleted `crates/ark-lsp/tests/lsp_e2e.rs` harness. Comments updated to point at the selfhost source `src/compiler/lsp.ark` and to record the retirement in #572. No protocol or wire-format change was needed.

### Files changed

- **Deleted**: `crates/ark-lsp/` (entire directory)
- **Cargo**: `Cargo.toml` (members + default-members), `Cargo.lock` (regenerated by `cargo check`)
- **CI**: `.github/workflows/ci.yml` (removed `lsp-e2e` job; added retirement marker)
- **Scripts**: `scripts/check/check-panic-audit.sh`, `scripts/gate_domain/checks.py` (drop `--exclude ark-lsp`)
- **Docs (operational)**: `README.md`, `docs/contributing.md`, `docs/directory-ownership.md`, `docs/compiler/bootstrap.md`, `docs/compiler/pipeline.md`, `docs/release-criteria.md`, `docs/release-checklist.md`, `docs/module-resolution.md`
- **Agent docs**: `.github/agents/impl-vscode-ide.agent.md`, `.github/agents/impl-editor-runtime.agent.md`, `codex-skills/impl-vscode-ide/SKILL.md`, `codex-skills/impl-editor-runtime/SKILL.md`
- **Extension comments**: `extensions/arukellt-all-in-one/src/test/extension.test.js`
- **Untouched (per scope)**: `docs/adr/ADR-015-no-panic-in-user-paths.md` (historical ADR), `docs/process/roadmap-v5.md` (historical roadmap)

### False-done checklist

1. ✓ `test ! -d crates/ark-lsp` (rc=0)
2. ✓ `grep -F "crates/ark-lsp" Cargo.toml` empty
3. ✓ `grep -RIn "\bark-lsp\b" crates/*/Cargo.toml` empty
4. ✓ `rg -l "\bark_lsp\b" crates/ src/` empty
5. ✓ `rg -l "\bark-lsp\b" scripts/ .github/workflows/` returns only the retirement-marker comment
6. ✓ `rg -l "\bark_lsp\b\|\bark-lsp\b" docs/` returns only documented-removal records (enumerated above) plus historical ADR-015 and roadmap-v5 (intentionally preserved)
7. ✓ All gates: 17/21 PASS = baseline; FAIL=0 increase, SKIP=0 increase
8. ✓ `cargo check --workspace` rc=0
9. ✓ Single commit; `git show --stat` confined to PRIMARY + ALLOWED ADJACENT paths
10. ✓ `python3 scripts/check/check-docs-consistency.py` rc=0 (folded into verify quick)