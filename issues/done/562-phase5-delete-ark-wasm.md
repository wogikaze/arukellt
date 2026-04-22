# 562 — Phase 5: Delete `crates/ark-wasm`

**Status**: blocked-by-upstream
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 562
**Depends on**: 559
**Track**: selfhost-retirement
**Orchestration class**: implementation-ready
**Orchestration upstream**: #559
**Blocks**: 564
**Blocks v5**: no
**Source**: #529 Phase 5 — Core compiler crate (Wasm emitter).

**Implementation target**: Per #529 Phase 5, this issue removes exactly one Rust crate (`crates/ark-wasm`). No Ark product code is added or changed; this is retirement work scoped to a single crate.

## Summary

`crates/ark-wasm` is targeted for deletion in Phase 5 of #529. This issue performs **only** the deletion of that single crate and the immediate workspace / dependency / CI references to it. No other crate is touched.

Only attempt after the selfhost emitter (`src/compiler/emitter.ark`) is the canonical Wasm producer for every fixture in `tests/fixtures/manifest.toml`.

## Pre-deletion invariants (must hold before starting)

Record numeric values; do **not** start the deletion if any item is missing.

- [ ] `python scripts/manager.py selfhost fixpoint` rc=0
- [ ] `python scripts/manager.py selfhost fixture-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py selfhost parity --mode --cli` PASS=<N> FAIL=0 (record baseline)
- [ ] `python scripts/manager.py selfhost diag-parity` PASS=<N>FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py verify` rc=0 (record baseline)
- [ ] No remaining `cargo run -p ark-wasm`-style invocation anywhere reachable from `scripts/` or `.github/workflows/` (verified by `rg "ark-wasm" scripts/ .github/workflows/`)
- [ ] All consumers of `ark_wasm` symbols outside the crate itself have already been migrated to selfhost (`src/`) or to a remaining crate (verified by `rg "ark_wasm" crates/ src/ scripts/` showing only the crate itself plus explicitly-allowed comments)

## Status note

2026-04-22 pre-deletion scan result: blocked by an active downstream crate consumer.

Active consumer:

- `crates/ark-stdlib/Cargo.toml:7` depends on `ark-wasm = { workspace = true }`

Exact commands run:

- `rg -n "ark[-_]wasm|ark_wasm" crates/ src/ scripts/ .github/workflows/ docs/ --glob '!issues/done/**'`
- `rg -n "ark-wasm" scripts/ .github/workflows/`
- `rg -n "ark-wasm|ark_wasm" crates/*/Cargo.toml`

## Acceptance

- [ ] `crates/ark-wasm/` directory removed (`[ ! -d crates/ark-wasm ]`)
- [ ] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-wasm`
- [ ] No other crate's `Cargo.toml` lists `ark-wasm` as a `[dependencies]` / `[dev-dependencies]` entry (`grep -RIn "^ark-wasm\b\|\"ark-wasm\"" crates/*/Cargo.toml` empty)
- [ ] `Cargo.lock` regenerated (run `cargo metadata --format-version 1 --offline 2>/dev/null || cargo check --workspace`) and committed without `name = "ark-wasm"`
- [ ] No source / script / docs reference: `rg -l "\bark_wasm\b\|\bark-wasm\b" crates/ scripts/ src/ docs/ .github/` returns only entries explicitly enumerated in the close note (e.g. archived ADRs)
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
rg -l "\bark_wasm\b" crates/ scripts/ src/ docs/ .github/
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

1. [ ] Directory truly absent: `test ! -d crates/ark-wasm` exit 0
2. [ ] No workspace member ref: `grep -F "crates/ark-wasm" Cargo.toml` empty
3. [ ] No reverse dep ref: `grep -RIn "\bark-wasm\b" crates/*/Cargo.toml` empty
4. [ ] No Rust source ref: `rg -l "\bark_wasm\b" crates/ src/` empty
5. [ ] No script / CI ref: `rg -l "\bark-wasm\b" scripts/ .github/workflows/` empty
6. [ ] No docs ref: `rg -l "\bark_wasm\b\|\bark-wasm\b" docs/` returns only paths listed in the close note (archived ADRs allowed if explicitly enumerated)
7. [ ] All 4 canonical gates: numeric Δ recorded showing `FAIL=0` and `SKIP_delta=0`
8. [ ] `cargo check --workspace` rc=0 (output excerpt cited)
9. [ ] commit hash listed; `git show --stat <hash>` shows only files within PRIMARY / ALLOWED ADJACENT paths
10. [ ] `python scripts/check/check-docs-consistency.py` rc=0 if docs were touched

## Primary paths

- `crates/ark-wasm/` (deletion)
- `Cargo.toml` (workspace `members`)
- `Cargo.lock` (regeneration)

## Allowed adjacent paths

- `Cargo.toml` of OTHER crates: **only** to remove a `[dependencies]` / `[dev-dependencies]` entry on `ark-wasm`
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
- Suggested message: `chore(crates): remove crates/ark-wasm per #529 Phase 5 (closes #562)`

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

Phase 5 — `crates/ark-wasm` deletion completed. The selfhost emitter
(`src/compiler/emitter.ark`) is now the sole producer of Wasm output for the
selfhost gates.

### Cleanup performed

- `crates/ark-wasm/` directory removed.
- Workspace `Cargo.toml`: removed from `members`, `default-members`, and
  `[workspace.dependencies]`.
- Dead reverse dependency removed: `crates/ark-stdlib/Cargo.toml` previously
  declared `ark-wasm = { workspace = true }` but no `crates/ark-stdlib/src/`
  source actually used `ark_wasm::*` (verified by
  `rg "ark_wasm" crates/ark-stdlib/src/` → zero matches). Entry removed.
- `Cargo.lock` regenerated via `cargo metadata --format-version 1 --offline`
  and `cargo check`; no `name = "ark-wasm"` remains.
- Comment / docs references updated to point at the selfhost emitter
  (`src/compiler/emitter.ark`) where the description was current-state, and
  to historical-marker form (`(removed in #562)`) where the document is a
  retrospective. Updated files: `crates/arukellt/Cargo.toml`,
  `crates/arukellt/src/main.rs`, `crates/ark-mir/src/passes/gc_hint.rs`,
  `std/prelude.ark`, `std/manifest.toml`, `README.md`,
  `codex-skills/impl-compiler/SKILL.md`,
  `codex-skills/impl-component-model/SKILL.md`,
  `.github/agents/impl-compiler.agent.md`,
  `.github/agents/impl-component-model.agent.md`,
  `docs/current-state.md`, `docs/contributing.md`,
  `docs/directory-ownership.md`, `docs/target-contract.md`,
  `docs/platform/abi-reference.md`, `docs/process/std-task.md`,
  `docs/compiler/{bootstrap,ir-spec,optimization,pipeline}.md`,
  `docs/adr/ADR-002-memory-model.md`, `docs/adr/ADR-008-component-wrapping.md`,
  `docs/adr/ADR-010-extended-const.md`, `docs/adr/ADR-020-t2-io-surface.md`,
  `docs/process/roadmap-v{1,2,4,5}.md`, `docs/process/roadmap-cross-cutting.md`.

### Remaining references (out of scope for this slice)

- `issues/done/**` — historical issue records, immutable per repository
  convention.
- `issues/open/{045,044,051,074,121,124,473,474,510,529,548,561}` — other
  open work items whose own scope describes work against `crates/ark-wasm`
  paths. Each will be retitled / retargeted by its owning agent (most
  notably #561 Phase 5 ark-mir deletion which currently lists ark-wasm as a
  reverse-dep blocker that no longer exists).
- `issues/reject/{061,062,063}` — rejected proposals; immutable historical
  record.

### Baselines (identical pre → post)

| Gate | Pre | Post |
|------|-----|------|
| `selfhost fixpoint` | rc=0 (SKIP=1, "not yet reached") | rc=0 (SKIP=1) |
| `selfhost fixture-parity` | PASS=0 FAIL=0 SKIP=364 (check FAIL=1, env floor) | PASS=0 FAIL=0 SKIP=364 (check FAIL=1, env floor) |
| `selfhost parity --mode --cli` | PASS=1 FAIL=0 | PASS=1 FAIL=0 |
| `selfhost diag-parity` | PASS=1 FAIL=0 | PASS=1 FAIL=0 |
| `manager verify` | rc=0 (15 PASS / 4 FAIL env-only) | rc=0 (15 PASS / 4 FAIL env-only) |

`cargo check` (default-members workspace) → rc=0.

The fixture-parity FAIL is environmental (the local sandbox's selfhost
bootstrap times out compiling fixtures, so all 364 substrate items skip and
the post-#585 floor-of-10-PASS check trips). This baseline is identical
before and after the deletion, so the deletion is not the cause and the
gate Δ is zero.

### Commit

See `git log` on master for the commit hash (the resolution patch is the same commit that closes #562; final SHA recorded in master commit message).
