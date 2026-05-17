---
Status: done
Created: 2026-04-22
Updated: 2026-05-17
ID: 577
Track: selfhost-retirement
Depends on: 564, 631
Orchestration class: completed
Blocks: 575, 576, 578, 579, 582
Blocks v5: no
Source: "#529 Phase 7 — Rust typechecker crate replaced by `src/compiler/typechecker.ark`."
Implementation target: "Remove exactly one Rust crate (`crates/ark-typecheck`) and immediate workspace/dependency/docs references."
REBUILD_BEFORE_VERIFY: "yes (workspace topology change forces selfhost rebuild)"
---

# 577 — Phase 7: Delete `crates/ark-typecheck`

## Summary

Closed 2026-05-17. The retired Rust `crates/ark-typecheck` crate was removed
from the workspace. Current typechecking source-of-truth is selfhost
`src/compiler/typechecker.ark`.

## Acceptance

- [x] `crates/ark-typecheck/` directory removed (`[ ! -d crates/ark-typecheck ]`)
- [x] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-typecheck`
- [x] No other crate's `Cargo.toml` lists `ark-typecheck` as a dependency
- [x] `Cargo.lock` regenerated and contains no `name = "ark-typecheck"`
- [x] No source / script / docs reference remains for `ark_typecheck`, `ark-typecheck`, or `crates/ark-typecheck`
- [x] `python scripts/manager.py verify` passes
- [x] 4 canonical selfhost gates pass with FAIL=0 and no SKIP increase

## Close Notes

- Removed `crates/ark-typecheck/`.
- Removed `crates/ark-typecheck` from `Cargo.toml` workspace members,
  default-members, and workspace dependencies.
- Regenerated `Cargo.lock`.
- Updated `scripts/check/check-stdlib-manifest.sh` to validate
  `std/manifest.toml` against `std/prelude.ark` without reading the retired
  Rust `builtins.rs`.
- Updated docs to point typechecker references at `src/compiler/typechecker.ark`
  or remove the retired crate from current crate lists.

## Verification Results

- `python scripts/manager.py verify`: PASS (23 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixpoint`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixture-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost parity --mode --cli`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost diag-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `cargo check --workspace`: PASS
- `bash scripts/check/check-stdlib-manifest.sh`: PASS
- `rg -l "\bark_typecheck\b|\bark-typecheck\b|crates/ark-typecheck|name = \"ark-typecheck\"" Cargo.toml Cargo.lock crates/ scripts/ src/ docs/ .github/`: no files
- `test ! -d crates/ark-typecheck`: PASS
- `grep -F "crates/ark-typecheck" Cargo.toml`: no output
- `grep -RIn "\bark-typecheck\b" crates/*/Cargo.toml`: no output

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
