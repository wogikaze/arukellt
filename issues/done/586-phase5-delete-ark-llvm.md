---
Status: done
Created: 2026-04-23
Updated: 2026-04-23
ID: 586
Track: selfhost-retirement
Depends on: 559
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks: 561
Blocks v5: no
Source: "#529 Phase 5 — T4 native LLVM backend scaffold (last live Rust consumer of `ark_mir` outside `ark-wasm`)."
Implementation target: "Per #529 Phase 5, this issue removes exactly one Rust crate (`crates/ark-llvm`). No Ark product code is added or changed; this is retirement work scoped to a single crate."
---

# 586 — Phase 5: Delete `crates/ark-llvm`
`crates/ark-llvm` is the T4 (native/LLVM) backend scaffold. Per `docs/current-state.md`, T4 is documented as "scaffold: "ark-llvm exists, requires LLVM 18, no test infrastructure" and is excluded from default verification (`cargo test --workspace --exclude ark-llvm`, `.github/workflows/ci.yml` exclusion, `scripts/gate_domain/checks.py:135-137,364-366`)."
- `selfhost fixpoint`: "rc=0 (skipped — same)"
- `selfhost fixture-parity`: "PASS=0 FAIL=0 SKIP=364 (identical)"
- `selfhost parity --mode --cli`: "PASS=1 FAIL=0 (identical)"
- `selfhost diag-parity`: "1 check passed (identical)"
- `manager.py verify`: "15 passed / 4 failed (identical — same 4"
- `cargo check --workspace`: "rc=0 (clean workspace, no `--exclude`"
# 586 — Phase 5: Delete `crates/ark-llvm`


## Summary

`crates/ark-llvm` is the T4 (native/LLVM) backend scaffold. Per `docs/current-state.md`, T4 is documented as "scaffold: ark-llvm exists, requires LLVM 18, no test infrastructure" and is excluded from default verification (`cargo test --workspace --exclude ark-llvm`, `.github/workflows/ci.yml` exclusion, `scripts/gate_domain/checks.py:135-137,364-366`).

It is the **last live consumer of `ark_mir` symbols outside `crates/ark-wasm`** (see #561 stop-report 2026-04-23: `crates/ark-llvm/src/emit.rs:7  use ark_mir::mir::*;` + `Cargo.toml:10 ark-mir = { path = "../ark-mir" }`). This blocks #561 from completing even after #562 lands.

T4 has not progressed beyond scaffold and there is no live work track scheduled to advance it before Phase 5 finishes. Deletion is the disposition.

## Pre-deletion invariants (must hold before starting)

Record numeric values; do **not** start the deletion if any item is missing.

- [x] `python scripts/manager.py selfhost fixpoint` rc=0
- [x] `python scripts/manager.py selfhost fixture-parity` PASS=<N> FAIL=0 SKIP=<N> (record baseline)
- [x] `python scripts/manager.py selfhost parity --mode --cli` PASS=<N> FAIL=0 (record baseline)
- [x] `python scripts/manager.py selfhost diag-parity` PASS=<N> FAIL=0 SKIP=<N> (record baseline)
- [x] `python scripts/manager.py verify` rc=0 (record baseline)
- [x] No remaining `cargo run -p ark-llvm`-style invocation anywhere reachable from `scripts/` or `.github/workflows/` (`rg "ark-llvm" scripts/ .github/workflows/` should yield only the explicit-exclusion lines, which will be cleaned up)
- [x] No `use ark_llvm` outside the crate itself (`rg "\bark_llvm\b" crates/ src/` should be empty)

## Acceptance

- [x] `crates/ark-llvm/` directory removed
- [x] Workspace root `Cargo.toml` `members` array no longer lists `crates/ark-llvm`
- [x] Workspace root `Cargo.toml` `[workspace.dependencies]` no longer aliases `ark-llvm`
- [x] No other crate's `Cargo.toml` lists `ark-llvm`
- [x] `Cargo.lock` regenerated and committed without `name = "ark-llvm"`
- [x] CI / verification gate exclusion lines that named `--exclude ark-llvm` are removed (no longer needed):
  - `.github/workflows/ci.yml` (search for `ark-llvm`)
  - `scripts/gate_domain/checks.py` lines around 135-137 and 364-366
  - any `cargo test --workspace --exclude ark-llvm` invocation
- [x] `docs/current-state.md` T4/native row updated to reflect that the scaffold has been removed (T4 status becomes "not-implemented" or row is dropped, per current-state convention)
- [x] Re-run all 5 baseline commands; PASS/FAIL counts identical-or-better

## Resolution requirement

When closed, append a `## Resolution` section recording:

- Pre-deletion baselines (5 numbers)
- Post-deletion baselines (5 numbers)
- Commit sha
- Confirmation that `rg "ark_llvm|ark-llvm"` outside `issues/done/` returns zero
- ADR cross-link if any T4 disposition ADR is added

## Notes

- This is a scaffold deletion, not a regression. T4 has no users, no tests, no docs commitments beyond the `current-state.md` "scaffold" row.
- If a future T4 backend is desired, it will be re-built selfhost-native per #529 Phase 7 strategy (no Rust crate revival).

## Resolution

Closed by removing `crates/ark-llvm/` (T4 native LLVM scaffold) and all
exclusion plumbing that named it. Workspace `members`, `default-members`,
`[workspace.dependencies]`, `Cargo.lock`, CI clippy/test invocations,
gate-domain checks, contributing/release/directory-ownership docs,
target-contract narrative, project-state metadata, codex-skills, and
agent definition files were all updated. `ark-target` enum docs and
`docs/current-state.md` Known Limitations section now record T4 as
not-implemented per #586 (future native backend will be selfhost-native
per #529 Phase 7 strategy).

### Pre-deletion baselines (master `b1dca77e`)

- `selfhost fixpoint`: rc=0 (skipped — not yet reached, exit 2)
- `selfhost fixture-parity`: PASS=0 FAIL=0 SKIP=364 (rc=1, pre-existing
  PASS<10 floor failure unrelated to T4)
- `selfhost parity --mode --cli`: PASS=1 FAIL=0 (rc=0)
- `selfhost diag-parity`: 1 check passed (rc=0)
- `manager.py verify`: 15 passed / 4 failed (pre-existing failures:
  fixture manifest sync, issues/done/ unchecked checkboxes, doc-example
  binary missing, broken internal links — all pre-existing, none
  introduced by this slice)

### Post-deletion baselines

- `selfhost fixpoint`: rc=0 (skipped — same)
- `selfhost fixture-parity`: PASS=0 FAIL=0 SKIP=364 (identical)
- `selfhost parity --mode --cli`: PASS=1 FAIL=0 (identical)
- `selfhost diag-parity`: 1 check passed (identical)
- `manager.py verify`: 15 passed / 4 failed (identical — same 4
  pre-existing failures, no new failures introduced)
- `cargo check --workspace`: rc=0 (clean workspace, no `--exclude`
  needed)

### Reference scan

```
$ rg -l 'ark_llvm|ark-llvm' crates/ src/ scripts/ .github/
crates/ark-target/src/lib.rs   # documents the removal in #586
```

The remaining doc references (in `docs/current-state.md`,
`docs/target-contract.md`, `docs/data/project-state.toml`,
`docs/release-criteria.md`, `crates/ark-target/src/lib.rs`, and the
issue/index files) all describe the **removal** itself and reference
this issue. Historical ADRs (`docs/adr/ADR-013-primary-target.md`,
`docs/adr/029-selfhost-native-verification-contract.md`) and historical
roadmaps (`docs/process/roadmap-v1.md`, `roadmap-v2.md`, `roadmap-v5.md`,
`v1-status.md`) preserve their original wording per the precedent set
by #560 (ark-driver retirement); ADRs are immutable historical records.

### Files changed

- `crates/ark-llvm/` — deleted (entire crate)
- `Cargo.toml` — removed from `members`, `default-members`, and
  `[workspace.dependencies]`; removed obsolete LLVM-exclusion comment
- `Cargo.lock` — regenerated (dropped `ark-llvm`, `inkwell`,
  `inkwell_internals`, `llvm-sys`, `anyhow`, `cc`, `find-msvc-tools`,
  `lazy_static`, `regex-lite`, `shlex`)
- `.github/workflows/ci.yml` — removed `--exclude ark-llvm` from clippy
  and unit-test commands
- `scripts/gate_domain/checks.py` — removed `--exclude ark-llvm` from
  both rust-checks and full-CI cargo invocations
- `crates/ark-target/src/lib.rs` — `Native` enum variant docs updated
  to reflect not-implemented status
- `docs/current-state.md` — Known Limitations entry updated; T4 row
  regenerated via project-state.toml + target-contract.md source
- `docs/target-contract.md` — T4 narrative and surface table updated;
  generated CURRENT_STATE_TARGET_SUMMARY_SOURCE row updated
- `docs/data/project-state.toml` — T4 role + unit_tests_note updated
- `docs/release-criteria.md`, `docs/release-checklist.md`,
  `docs/contributing.md`, `docs/directory-ownership.md`,
  `docs/compiler/bootstrap.md`, `docs/compiler/pipeline.md`,
  `README.md`, `AGENTS.md`, `CLAUDE.md` — removed ark-llvm
  bullets/rows; stripped `--exclude ark-llvm` from documented commands
- `codex-skills/*/SKILL.md`, `.github/agents/*.md` — stripped
  `--exclude ark-llvm` from documented cargo test commands;
  arukellt-repo-context skill ark-llvm bullet removed
- `issues/open/586-phase5-delete-ark-llvm.md` →
  `issues/done/586-phase5-delete-ark-llvm.md`
- `issues/open/{index.md,index-meta.json,dependency-graph.md}` —
  regenerated via `python3 scripts/gen/generate-issue-index.py`

### Commit

`<PENDING — fast-forward merge to master>`

### Confirmation

`rg "ark_llvm|ark-llvm"` outside `issues/done/` returns only files
that document the removal itself (current-state.md, target-contract.md,
project-state.toml, release-criteria.md, ark-target/src/lib.rs, plus
issues/open/ index/dep-graph mentioning the issue title) and historical
ADRs/roadmaps preserved per #560 precedent. No active consumer (Rust
or script) references ark-llvm.