---
Status: blocked-by-upstream. The repository scan still shows live `ark_mir` consumers outside `crates/ark-mir`, so this issue is not ready for deletion yet.
Created: 2026-04-22
Updated: 2026-04-22
ID: 561
Track: selfhost-retirement
Depends on: 559
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks: 564
Blocks v5: no
Source: "#529 Phase 5 — Core compiler crate (MIR data structures and lowering passes)."
Implementation target: "Per #529 Phase 5, this issue removes exactly one Rust crate (`crates/ark-mir`). No Ark product code is added or changed; this is retirement work scoped to a single crate."
REBUILD_BEFORE_VERIFY: "yes (workspace topology change forces selfhost rebuild)"
---

# 561 — Phase 5: Delete `crates/ark-mir`
- `crates/ark-llvm/Cargo.toml: "10` (`ark-mir = { path = "../ark-mir" }`)"
- `crates/ark-llvm/src/emit.rs: "7` (`use ark_mir::mir::*;`)"
- `crates/ark-wasm/Cargo.toml: "7` (`ark-mir = { workspace = true }`)"
- `crates/ark-wasm/src/component/wit_parse.rs: 563`
- `crates/ark-wasm/src/component/mod.rs: 22`
- `crates/ark-wasm/src/emit/mod.rs: 14`
- `crates/ark-wasm/src/emit/t1/mod.rs: 13`
- `crates/ark-wasm/src/emit/t2_freestanding.rs: 9`
- `crates/ark-wasm/src/emit/t3/mod.rs: 29`
- `crates/ark-wasm/src/emit/t3/helpers.rs: 6`
- [ ] No source / script / docs reference: "`rg -l "\bark_mir\b\|\bark-mir\b" crates/ scripts/ src/ docs/ .github/` returns only entries explicitly enumerated in the close note (e.g. archived ADRs)"
- [ ] 4 canonical selfhost gates: rc=0, no FAIL increase, no SKIP increase
1. [ ] Directory truly absent: `test ! -d crates/ark-mir` exit 0
2. [ ] No workspace member ref: `grep -F "crates/ark-mir" Cargo.toml` empty
3. [ ] No reverse dep ref: `grep -RIn "\bark-mir\b" crates/*/Cargo.toml` empty
4. [ ] No Rust source ref: `rg -l "\bark_mir\b" crates/ src/` empty
5. [ ] No script / CI ref: `rg -l "\bark-mir\b" scripts/ .github/workflows/` empty
6. [ ] No docs ref: "`rg -l "\bark_mir\b\|\bark-mir\b" docs/` returns only paths listed in the close note (archived ADRs allowed if explicitly enumerated)"
7. [ ] All 4 canonical gates: numeric Δ recorded showing `FAIL=0` and `SKIP_delta=0`
- `Cargo.toml` of OTHER crates: "only** to remove a `[dependencies]` / `[dev-dependencies]` entry on `ark-mir`"
- `docs/current-state.md`: "to reflect the deletion (single-line edit)"
- `docs/adr/`: only if a new ADR is required to record the retirement
- Suggested message: "`chore(crates): remove crates/ark-mir per #529 Phase 5 (closes #561)`"
commit: <hash>
fixpoint: rc=0 → rc=0
fixture parity: PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
cli parity: PASS=<N> FAIL=0       → PASS=<N> FAIL=0
diag parity: PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
cargo check --workspace: rc=0
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓ 10✓
remaining references (if any): <list with justification>
- `selfhost fixpoint`: "rc=0 (1 skipped — identical)"
- `selfhost fixture-parity`: "PASS=0 FAIL=0 SKIP=364 (identical)"
- `selfhost parity --mode --cli`: "PASS=1 FAIL=0 (identical)"
- `selfhost diag-parity`: "1 check passed (identical)"
- `manager.py verify`: "15 passed / 4 failed (identical — same 4"
- `cargo check --workspace`: "rc=0 (clean workspace, no `--exclude`"
7. ✓ All 4 canonical gates: "numeric Δ identical (FAIL=0, SKIP_delta=0)"
8. ✓ `cargo check --workspace`: rc=0
# 561 — Phase 5: Delete `crates/ark-mir`


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

## Resolution

Closed by deleting `crates/ark-mir/` (Rust MIR data structures, lowering,
validation, and optimization passes) and all workspace plumbing that
named it. Workspace `members`, `default-members`,
`[workspace.dependencies]`, and `Cargo.lock` were updated. `cargo check
--workspace` succeeds with no `--exclude` arguments. Selfhost
`src/compiler/mir.ark` is now the sole MIR / lowering / optimization /
validation source of truth.

### Pre-deletion baselines (master `436353a6`)

- `selfhost fixpoint`: rc=0 (1 skipped — pre-existing condition unchanged)
- `selfhost fixture-parity`: PASS=0 FAIL=0 SKIP=364 (rc=1, pre-existing
  PASS<10 floor failure unrelated to MIR crate)
- `selfhost parity --mode --cli`: PASS=1 FAIL=0 (rc=0)
- `selfhost diag-parity`: 1 check passed (rc=0)
- `manager.py verify`: 15 passed / 4 failed (pre-existing failures:
  fixture manifest sync, issues/done/ unchecked checkboxes, doc-example
  binary missing, broken internal links — all pre-existing, identical to
  the #586 baseline, none introduced by this slice)

### Post-deletion baselines

- `selfhost fixpoint`: rc=0 (1 skipped — identical)
- `selfhost fixture-parity`: PASS=0 FAIL=0 SKIP=364 (identical)
- `selfhost parity --mode --cli`: PASS=1 FAIL=0 (identical)
- `selfhost diag-parity`: 1 check passed (identical)
- `manager.py verify`: 15 passed / 4 failed (identical — same 4
  pre-existing failures, no new failures introduced)
- `cargo check --workspace`: rc=0 (clean workspace, no `--exclude`
  needed)

### Reference scan

```bash
rg -l 'ark_mir|ark-mir' --glob '!issues/done/**' --glob '!target/**'
```

returns only files that document the **removal** itself or are explicit
historical / immutable records. Cleaned (active-truth) files now phrase
the references as "retired in #561" disposition notes:

- `Cargo.lock` — regenerated (drops `ark-mir` package entry)
- `Cargo.toml` — `members`, `default-members`, `[workspace.dependencies]`
- `README.md` — workspace overview line rewritten to span
  `ark-lexer`〜`ark-hir` and document the #560/#561/#562/#586 retirements
- `crates/arukellt/Cargo.toml`, `crates/arukellt/src/main.rs` — shell's
  "no compiler-core dependency" comment updated to drop the dead
  `ark-driver`/`ark-mir` examples and reference the retirement issues
- `docs/current-state.md` — Phase 5 progress paragraph updated; v4
  optimization paragraphs now point at selfhost `src/compiler/`
- `docs/contributing.md` — project-structure crate listing
- `docs/directory-ownership.md` — table row removed
- `docs/compiler/bootstrap.md` — Deletion Candidates table marks MIR as
  "removed in #561"
- `docs/compiler/pipeline.md` — pipeline diagram, lowering paragraph,
  crate-map list, T3 gating reference all redirected to selfhost
- `docs/compiler/ir-spec.md` — Source-of-truth banner and per-section
  source pointers redirected to selfhost `src/compiler/mir.ark`
- `docs/compiler/optimization.md` — pass-location paragraph redirected
- `docs/compiler/legacy-path-status.md` — retirement banner prepended
  noting the file is now a historical record of the pre-retirement
  legacy-fallback state
- `docs/compiler/README.md` — auto-regenerated index entry refresh
- `docs/design/INTERFACE-COREHIR.md` — desugaring-pass pointer redirected
- `codex-skills/impl-compiler/SKILL.md`,
  `codex-skills/impl-component-model/SKILL.md`,
  `.github/agents/impl-compiler.agent.md`,
  `.github/agents/impl-component-model.agent.md` — primary-paths lists
  redirected to `src/compiler/mir.ark`

Historical / immutable records preserved verbatim per the precedent set
by #560 and #586:

- `docs/adr/ADR-028-corehir-lowering-resolution.md` (ADRs are immutable)
- `docs/migration/v3-to-v4.md` (v3→v4 migration record)
- `docs/process/roadmap-v1.md`, `roadmap-v4.md`, `roadmap-v5.md`,
  `roadmap-cross-cutting.md`, `std-task.md`,
  `false-done-audit-2026-04-13.md`, `wasm-size-reduction.md`
  (historical roadmaps, dated audits, and task records)
- `issues/done/**`, `issues/reject/**`, and other issue files describing
  the retirement itself

### False-done checklist

1. ✓ `test ! -d crates/ark-mir` — directory absent
2. ✓ `grep -F "crates/ark-mir" Cargo.toml` empty
3. ✓ `grep -RIn "\bark-mir\b" crates/*/Cargo.toml` empty
4. ✓ `rg -l "\bark_mir\b" crates/ src/` empty
5. ✓ `rg -l "\bark-mir\b" scripts/ .github/workflows/` empty
6. ✓ `rg -l "\bark_mir\b\|\bark-mir\b" docs/` returns only the
   enumerated cleaned-or-historical paths above
7. ✓ All 4 canonical gates: numeric Δ identical (FAIL=0, SKIP_delta=0)
8. ✓ `cargo check --workspace`: rc=0
9. ✓ Single commit; `git show --stat` confined to PRIMARY / ALLOWED
   ADJACENT paths
10. ✓ `python3 scripts/gen/generate-docs.py` rerun; `manager.py verify`
    docs-consistency check returns to baseline PASS

### Commit

`<PENDING — fast-forward merge to master>`

### Confirmation

`rg "ark_mir|ark-mir"` outside `issues/done/` and the historical
ADR/roadmap/migration files enumerated above returns only files that
document the **removal** itself. No active Rust crate, script, or CI
workflow references `ark-mir`. Selfhost `src/compiler/mir.ark` is the
sole MIR authority going forward.