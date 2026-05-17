---
Status: done
Created: 2026-04-22
Updated: 2026-05-17
ID: 576
Track: selfhost-retirement
Depends on: 564, 631, 577
Orchestration class: completed
Blocks: 574, 575, 579, 581, 582
Blocks v5: no
Source: "#529 Phase 7 — Rust resolver crate replaced by `src/compiler/resolver.ark`."
Implementation target: "Remove exactly one Rust crate (`crates/ark-resolve`) and immediate workspace/dependency/docs references."
REBUILD_BEFORE_VERIFY: "yes (workspace topology change forces selfhost rebuild)"
---

# 576 — Phase 7: Delete `crates/ark-resolve`

## Summary

Closed 2026-05-17. The retired Rust `crates/ark-resolve` crate was removed
from the workspace. Current resolver source-of-truth is selfhost
`src/compiler/resolver.ark`, with manifest/project command handling in
`src/compiler/main.ark`.

## Acceptance

- [x] `crates/ark-resolve/` directory removed (`[ ! -d crates/ark-resolve ]`)
- [x] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-resolve`
- [x] No other crate's `Cargo.toml` lists `ark-resolve` as a dependency
- [x] `Cargo.lock` regenerated and contains no `name = "ark-resolve"`
- [x] No source / script / docs reference remains for `ark_resolve`, `ark-resolve`, or `crates/ark-resolve`
- [x] `python scripts/manager.py verify` passes
- [x] 4 canonical selfhost gates pass with FAIL=0 and no SKIP increase

## Close Notes

- Removed `crates/ark-resolve/`.
- Removed `crates/ark-resolve` from `Cargo.toml` workspace members,
  default-members, and workspace dependencies.
- Regenerated `Cargo.lock`.
- Updated current docs to reference `src/compiler/resolver.ark` and
  `src/compiler/main.ark` instead of the retired Rust resolver crate.

## Verification Results

- `python scripts/manager.py verify`: PASS (23 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixpoint`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixture-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost parity --mode --cli`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost diag-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `cargo check --workspace`: PASS
- `rg -l "\bark_resolve\b|\bark-resolve\b|crates/ark-resolve|name = \"ark-resolve\"" Cargo.toml Cargo.lock crates/ scripts/ src/ docs/ .github/`: no files
- `test ! -d crates/ark-resolve`: PASS
- `grep -F "crates/ark-resolve" Cargo.toml`: no output
- `grep -RIn "\bark-resolve\b" crates/*/Cargo.toml`: no output

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
