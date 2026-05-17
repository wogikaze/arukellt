---
Status: done
Created: 2026-04-22
Updated: 2026-05-17
ID: 578
Track: selfhost-retirement
Depends on: 564, 577
Orchestration class: completed
Blocks: 579, 582
Blocks v5: no
Source: "#529 Phase 7 — Rust HIR crate replaced by selfhost CoreHIR/MIR pipeline."
Implementation target: "Remove exactly one Rust crate (`crates/ark-hir`) and immediate workspace/dependency/docs references."
REBUILD_BEFORE_VERIFY: "yes (workspace topology change forces selfhost rebuild)"
---

# 578 — Phase 7: Delete `crates/ark-hir`

## Summary

Closed 2026-05-17. The retired Rust `crates/ark-hir` crate was removed from
the workspace. Current CoreHIR/HIR-facing documentation points at selfhost
`src/compiler/corehir.ark` and `src/compiler/typechecker.ark`.

## Acceptance

- [x] `crates/ark-hir/` directory removed (`[ ! -d crates/ark-hir ]`)
- [x] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-hir`
- [x] No other crate's `Cargo.toml` lists `ark-hir` as a dependency
- [x] `Cargo.lock` regenerated and contains no `name = "ark-hir"`
- [x] No source / script / docs reference remains for `ark_hir`, `ark-hir`, or `crates/ark-hir`
- [x] `python scripts/manager.py verify` passes
- [x] 4 canonical selfhost gates pass with FAIL=0 and no SKIP increase

## Close Notes

- Removed `crates/ark-hir/`.
- Removed `crates/ark-hir` from `Cargo.toml` workspace members,
  default-members, and workspace dependencies.
- Regenerated `Cargo.lock`.
- Updated CoreHIR/HIR docs to reference selfhost `src/compiler/corehir.ark`.

## Verification Results

- `python scripts/manager.py verify`: PASS (23 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixpoint`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixture-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost parity --mode --cli`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost diag-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `cargo check --workspace`: PASS
- `rg -l "\bark_hir\b|\bark-hir\b|crates/ark-hir|name = \"ark-hir\"" Cargo.toml Cargo.lock crates/ scripts/ src/ docs/ .github/`: no files
- `test ! -d crates/ark-hir`: PASS
- `grep -F "crates/ark-hir" Cargo.toml`: no output
- `grep -RIn "\bark-hir\b" crates/*/Cargo.toml`: no output

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
