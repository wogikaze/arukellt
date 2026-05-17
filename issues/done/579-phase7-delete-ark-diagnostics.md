---
Status: done
Created: 2026-04-22
Updated: 2026-05-17
ID: 579
Track: selfhost-retirement
Depends on: 564, 572, 574, 575, 576, 577, 578, 581
Orchestration class: completed
Blocks: 582
Blocks v5: no
Source: "#529 Phase 7 — Rust diagnostics crate replaced by selfhost diagnostics in `src/compiler/`."
Implementation target: "Remove exactly one Rust crate (`crates/ark-diagnostics`) and immediate workspace/dependency/docs references."
REBUILD_BEFORE_VERIFY: "yes (workspace topology change forces selfhost rebuild)"
---

# 579 — Phase 7: Delete `crates/ark-diagnostics`

## Summary

Closed 2026-05-17. The retired Rust `crates/ark-diagnostics` crate was removed
from the workspace. Current diagnostics source-of-truth is selfhost
`src/compiler/diagnostics.ark`.

## Acceptance

- [x] `crates/ark-diagnostics/` directory removed (`[ ! -d crates/ark-diagnostics ]`)
- [x] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-diagnostics`
- [x] No other crate's `Cargo.toml` lists `ark-diagnostics` as a dependency
- [x] `Cargo.lock` regenerated and contains no `name = "ark-diagnostics"`
- [x] No source / script / docs reference remains for `ark_diagnostics`, `ark-diagnostics`, or `crates/ark-diagnostics`
- [x] `python scripts/manager.py verify` passes
- [x] 4 canonical selfhost gates pass with FAIL=0 and no SKIP increase

## Close Notes

- Removed `crates/ark-diagnostics/`.
- Removed `crates/ark-diagnostics` from `Cargo.toml` workspace members,
  default-members, and workspace dependencies.
- Regenerated `Cargo.lock`.
- Updated `scripts/check/check-diagnostic-codes.sh` to validate documented
  diagnostic codes against `src/compiler/`.
- Moved the full diagnostic-code registry into `src/compiler/diagnostics.ark`
  and updated docs to point at the selfhost diagnostics source.
- Removed generated `docs/playground/dist/` output because its CSS class names
  include `ark-diagnostics`; those are generated assets, not source refs.

## Verification Results

- `python scripts/manager.py verify`: PASS (23 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixpoint`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixture-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost parity --mode --cli`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost diag-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `bash scripts/check/check-diagnostic-codes.sh`: PASS (50 codes aligned)
- `cargo check --workspace`: not applicable after the final crate removal; the empty virtual workspace reports `contains no package`, and #582 removes `Cargo.toml` / `Cargo.lock`.
- `rg -l "\bark_diagnostics\b|\bark-diagnostics\b|crates/ark-diagnostics|name = \"ark-diagnostics\"" Cargo.toml Cargo.lock crates/ scripts/ src/ docs/ .github/`: no files
- `test ! -d crates/ark-diagnostics`: PASS
- `grep -F "crates/ark-diagnostics" Cargo.toml`: no output
- `grep -RIn "\bark-diagnostics\b" crates/*/Cargo.toml`: no output

## False-Done Checklist

1. [x] Directory truly absent
2. [x] No workspace member ref
3. [x] No reverse dependency ref
4. [x] No Rust source ref
5. [x] No script / CI ref
6. [x] No docs ref
7. [x] All 4 canonical gates pass with no FAIL/SKIP increase
8. [x] Cargo workspace is empty and ready for #582 final removal
9. [x] File set is limited to the crate deletion and immediate allowed references
10. [x] Docs consistency covered by `python scripts/manager.py verify`
