---
Status: done
Created: 2026-04-22
Updated: 2026-05-17
ID: 575
Track: selfhost-retirement
Depends on: 564, 631, 576, 577
Orchestration class: completed
Blocks: 574, 579, 582
Blocks v5: no
Source: "#529 Phase 7 — Rust parser crate replaced by `src/compiler/parser.ark`."
Implementation target: "Remove exactly one Rust crate (`crates/ark-parser`) and immediate workspace/dependency/docs references."
REBUILD_BEFORE_VERIFY: "yes (workspace topology change forces selfhost rebuild)"
---

# 575 — Phase 7: Delete `crates/ark-parser`

## Summary

Closed 2026-05-17. The retired Rust `crates/ark-parser` crate was removed from
the workspace. Current parser source-of-truth is selfhost
`src/compiler/parser.ark`.

## Acceptance

- [x] `crates/ark-parser/` directory removed (`[ ! -d crates/ark-parser ]`)
- [x] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-parser`
- [x] No other crate's `Cargo.toml` lists `ark-parser` as a dependency
- [x] `Cargo.lock` regenerated and contains no `name = "ark-parser"`
- [x] No source / script / docs reference remains for `ark_parser`, `ark-parser`, or `crates/ark-parser`
- [x] `python scripts/manager.py verify` passes
- [x] 4 canonical selfhost gates pass with FAIL=0 and no SKIP increase

## Close Notes

- Removed `crates/ark-parser/`.
- Removed `crates/ark-parser` from `Cargo.toml` workspace members,
  default-members, and workspace dependencies.
- Regenerated `Cargo.lock`.
- Updated current docs to reference selfhost `src/compiler/parser.ark` or the
  selfhost formatter.

## Verification Results

- `python scripts/manager.py verify`: PASS (23 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixpoint`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixture-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost parity --mode --cli`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost diag-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `cargo check --workspace`: PASS
- `rg -l "\bark_parser\b|\bark-parser\b|crates/ark-parser|name = \"ark-parser\"" Cargo.toml Cargo.lock crates/ scripts/ src/ docs/ .github/`: no files
- `test ! -d crates/ark-parser`: PASS
- `grep -F "crates/ark-parser" Cargo.toml`: no output
- `grep -RIn "\bark-parser\b" crates/*/Cargo.toml`: no output

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
