---
Status: done
Created: 2026-04-22
Updated: 2026-05-17
ID: 581
Track: selfhost-retirement
Depends on: 564, 576
Orchestration class: completed
Blocks: 579, 582
Blocks v5: no
Source: "#529 Phase 7 — Rust target-config crate replaced by selfhost target handling."
Implementation target: "Remove exactly one Rust crate (`crates/ark-target`) and immediate workspace/dependency/docs references."
REBUILD_BEFORE_VERIFY: "yes (workspace topology change forces selfhost rebuild)"
---

# 581 — Phase 7: Delete `crates/ark-target`

## Summary

Closed 2026-05-17. The retired Rust `crates/ark-target` crate was removed from
the workspace. Current target planning and target-id handling lives in selfhost
`src/compiler/driver.ark` and emitter validation paths.

## Acceptance

- [x] `crates/ark-target/` directory removed (`[ ! -d crates/ark-target ]`)
- [x] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-target`
- [x] No other crate's `Cargo.toml` lists `ark-target` as a dependency
- [x] `Cargo.lock` regenerated and contains no `name = "ark-target"`
- [x] No source / script / docs reference remains for `ark_target`, `ark-target`, or `crates/ark-target`
- [x] `python scripts/manager.py verify` passes
- [x] 4 canonical selfhost gates pass with FAIL=0 and no SKIP increase

## Close Notes

- Removed `crates/ark-target/`.
- Removed `crates/ark-target` from `Cargo.toml` workspace members,
  default-members, and workspace dependencies.
- Regenerated `Cargo.lock`.
- Updated current target docs to reference selfhost `src/compiler/driver.ark`.

## Verification Results

- `python scripts/manager.py verify`: PASS (23 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixpoint`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixture-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost parity --mode --cli`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost diag-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `cargo check --workspace`: PASS
- `rg -l "\bark_target\b|\bark-target\b|crates/ark-target|name = \"ark-target\"" Cargo.toml Cargo.lock crates/ scripts/ src/ docs/ .github/`: no files
- `test ! -d crates/ark-target`: PASS
- `grep -F "crates/ark-target" Cargo.toml`: no output
- `grep -RIn "\bark-target\b" crates/*/Cargo.toml`: no output

## False-Done Checklist

1. [x] Directory truly absent
2. [x] No workspace member ref
3. [x] No reverse dependency ref
4. [x] No Rust source ref
5. [x] No script / CI ref
6. [x] No docs ref
7. [x] All 4 canonical gates pass with no FAIL/SKIP increase
8. [x] `cargo check --workspace` rc=0
9. [x] File set is limited to the crate deletion and immediate allowed references
10. [x] Docs consistency covered by `python scripts/manager.py verify`
