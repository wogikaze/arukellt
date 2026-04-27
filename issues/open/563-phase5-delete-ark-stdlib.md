---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 563
Track: selfhost-retirement
Depends on: 559
Orchestration class: implementation-ready
Orchestration upstream: None
---

# 563 — Phase 5: Delete `crates/ark-stdlib`
3. [ ] No reverse dep ref: `grep -RIn "\bark-stdlib\b" crates/*/Cargo.toml` empty
4. [ ] No Rust source ref: `rg -l "\bark_stdlib\b" crates/ src/` empty
5. [ ] No script / CI ref: `rg -l "\bark-stdlib\b" scripts/ .github/workflows/` empty
6. [ ] No docs ref: "`rg -l "\bark_stdlib\b\|\bark-stdlib\b" docs/` returns only paths listed in the close note (archived ADRs allowed if explicitly enumerated)"
7. [ ] All 4 canonical gates: numeric Δ recorded showing `FAIL=0` and `SKIP_delta=0`
- `Cargo.toml` of OTHER crates: "only** to remove a `[dependencies]` / `[dev-dependencies]` entry on `ark-stdlib`"
- `docs/current-state.md`: "to reflect the deletion (single-line edit)"
- `docs/adr/`: only if a new ADR is required to record the retirement
- Suggested message: "`chore(crates): remove crates/ark-stdlib per #529 Phase 5 (closes #563)`"
commit: <hash>
fixpoint: rc=0 → rc=0
fixture parity: PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
cli parity: PASS=<N> FAIL=0       → PASS=<N> FAIL=0
diag parity: PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
cargo check --workspace: rc=0
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓ 10✓
remaining references (if any): <list with justification>
Reverse-dependency scan: "`rg -n "ark[-_]stdlib" --glob '!Cargo.lock' --glob '!issues/done/**' --glob '!docs/adr/**'`"
- `crates/arukellt/Cargo.toml: "20` — `ark-stdlib = { workspace = true }` (workspace dep)"
- `crates/ark-lsp/Cargo.toml: 17` — `ark-stdlib = { path = "../ark-stdlib" }`
---
# 563 — Phase 5: Delete `crates/ark-stdlib`


## Summary

`crates/ark-stdlib` is targeted for deletion in Phase 5 of #529. This issue performs **only** the deletion of that single crate and the immediate workspace / dependency / CI references to it. No other crate is touched.

Only attempt after the manifest is consumed exclusively from `std/manifest.toml` by the selfhost driver, with no remaining Rust consumer of `ark_stdlib::StdlibManifest`.

## Pre-deletion invariants (must hold before starting)

Record numeric values; do **not** start the deletion if any item is missing.

- [ ] `python scripts/manager.py selfhost fixpoint` rc=0
- [ ] `python scripts/manager.py selfhost fixture-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py selfhost parity --mode --cli` PASS=<N> FAIL=0 (record baseline)
- [ ] `python scripts/manager.py selfhost diag-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py verify` rc=0 (record baseline)
- [ ] No remaining `cargo run -p ark-stdlib`-style invocation anywhere reachable from `scripts/` or `.github/workflows/` (verified by `rg "ark-stdlib" scripts/ .github/workflows/`)
- [ ] All consumers of `ark_stdlib` symbols outside the crate itself have already been migrated to selfhost (`src/`) or to a remaining crate (verified by `rg "ark_stdlib" crates/ src/ scripts/` showing only the crate itself plus explicitly-allowed comments)

## Acceptance

- [ ] `crates/ark-stdlib/` directory removed (`[ ! -d crates/ark-stdlib ]`)
- [ ] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-stdlib`
- [ ] No other crate's `Cargo.toml` lists `ark-stdlib` as a `[dependencies]` / `[dev-dependencies]` entry (`grep -RIn "^ark-stdlib\b\|\"ark-stdlib\"" crates/*/Cargo.toml` empty)
- [ ] `Cargo.lock` regenerated (run `cargo metadata --format-version 1 --offline 2>/dev/null || cargo check --workspace`) and committed without `name = "ark-stdlib"`
- [ ] No source / script / docs reference: `rg -l "\bark_stdlib\b\|\bark-stdlib\b" crates/ scripts/ src/ docs/ .github/` returns only entries explicitly enumerated in the close note (e.g. archived ADRs)
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
rg -l "\bark_stdlib\b" crates/ scripts/ src/ docs/ .github/
```


## STOP_IF

- Any consumer in another crate / script / workflow still references this crate at deletion time → open a focused migration issue, mark this one `blocked-by-upstream`, **STOP**.
- Removing the crate causes any of the 4 canonical gates to regress (FAIL>0 or SKIP delta > 0) → revert the deletion commit and **STOP**.
- Removing the crate causes any fixture in `tests/fixtures/` to fail → revert and **STOP**.
- `cargo check --workspace` fails after removal → revert and **STOP**.
- A reverse-dependency was missed and surfaces only in CI → revert and **STOP**.

## False-done prevention checklist (close-gate reviewer must verify all)

The reviewer is a **different agent** from the implementer (`verify-issue-closure`). Each line must be checked with command output cited in the close note.

1. [ ] Directory truly absent: `test ! -d crates/ark-stdlib` exit 0
2. [ ] No workspace member ref: `grep -F "crates/ark-stdlib" Cargo.toml` empty
3. [ ] No reverse dep ref: `grep -RIn "\bark-stdlib\b" crates/*/Cargo.toml` empty
4. [ ] No Rust source ref: `rg -l "\bark_stdlib\b" crates/ src/` empty
5. [ ] No script / CI ref: `rg -l "\bark-stdlib\b" scripts/ .github/workflows/` empty
6. [ ] No docs ref: `rg -l "\bark_stdlib\b\|\bark-stdlib\b" docs/` returns only paths listed in the close note (archived ADRs allowed if explicitly enumerated)
7. [ ] All 4 canonical gates: numeric Δ recorded showing `FAIL=0` and `SKIP_delta=0`
8. [ ] `cargo check --workspace` rc=0 (output excerpt cited)
9. [ ] commit hash listed; `git show --stat <hash>` shows only files within PRIMARY / ALLOWED ADJACENT paths
10. [ ] `python scripts/check/check-docs-consistency.py` rc=0 if docs were touched

## Primary paths

- `crates/ark-stdlib/` (deletion)
- `Cargo.toml` (workspace `members`)
- `Cargo.lock` (regeneration)

## Allowed adjacent paths

- `Cargo.toml` of OTHER crates: **only** to remove a `[dependencies]` / `[dev-dependencies]` entry on `ark-stdlib`
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
- Suggested message: `chore(crates): remove crates/ark-stdlib per #529 Phase 5 (closes #563)`

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

## Status (slice attempt 2026-04-22)

Pre-deletion invariant **failed**: live consumers of `ark_stdlib` symbols still exist outside the crate itself. Per the slice STOP_IF / FORBIDDEN_PATHS protocol, no deletion was performed; only this status note is committed.

Reverse-dependency scan: `rg -n "ark[-_]stdlib" --glob '!Cargo.lock' --glob '!issues/done/**' --glob '!docs/adr/**'`

Active Rust consumers blocking deletion:

- `crates/arukellt/Cargo.toml:20` — `ark-stdlib = { workspace = true }` (workspace dep)
- `crates/arukellt/src/cmd_doc.rs` — heavy use of `StdlibManifest`, `ManifestFunction`, `ManifestModule` across the entire `arukellt doc` subcommand (manifest load, search, JSON emit, tests). Roughly 20+ call sites.
- `crates/ark-lsp/Cargo.toml:17` — `ark-stdlib = { path = "../ark-stdlib" }`
- `crates/ark-lsp/src/server.rs` — pervasive use of `StdlibManifest` for hover, completion, diagnostics, and tests (~10+ method signatures plus load/repo lookup at startup).

`crates/arukellt` is in this slice's FORBIDDEN_PATHS, so its consumers cannot be migrated here. `crates/ark-lsp` is not explicitly forbidden, but its consumption surface is large enough that migration is its own work item, not a side-effect of crate deletion.

Doc / metadata references (informational; not blockers):

- `docs/compiler/bootstrap.md`, `docs/compiler/pipeline.md`, `docs/contributing.md`, `docs/directory-ownership.md`, `docs/process/std-task.md`
- `Cargo.toml` workspace `members` + `[workspace.dependencies]` line for `ark-stdlib`
- `issues/open/dependency-graph.md`, `issues/open/index.md`, `issues/open/index-meta.json`, `issues/open/529-100-percent-selfhost-transition-plan.md`

### Recommended sequencing

Before this slice can run, the following migrations must land as their own focused issues (one per consumer crate, retirement-track):

1. **Migrate `crates/arukellt::cmd_doc`** off `ark_stdlib::StdlibManifest`. Either:
   - Re-point `arukellt doc` at the selfhost-produced manifest artifact / docs JSON, or
   - Inline a minimal manifest reader local to `crates/arukellt` and drop the workspace dep.
2. **Migrate `crates/ark-lsp` server** off `ark_stdlib::StdlibManifest`. The LSP needs a structured manifest for hover/completion; pick a single replacement path (selfhost-emitted JSON, or a small in-tree TOML reader) and convert all call sites in one pass.
3. Once both consumer crates have zero `ark_stdlib` references and zero `ark-stdlib` Cargo deps, re-run this slice to delete `crates/ark-stdlib/`, the workspace member line, the workspace-dep line, the doc references, and regenerate `Cargo.lock`.

No commit was made other than this status note; `crates/ark-stdlib/` and `Cargo.toml` are unchanged. Issue remains **open**.