---
Status: done
Created: 2026-04-22
Updated: 2026-05-17
ID: 574
Track: selfhost-retirement
Depends on: 564, 631, 575, 576
Orchestration class: completed
Blocks: 579, 582
Blocks v5: no
Source: "#529 Phase 7 — Rust lexer crate replaced by `src/compiler/lexer.ark`."
Implementation target: "Remove exactly one Rust crate (`crates/ark-lexer`) and immediate workspace/dependency/docs references."
REBUILD_BEFORE_VERIFY: "yes (workspace topology change forces selfhost rebuild)"
---

# 574 — Phase 7: Delete `crates/ark-lexer`

## Summary

Closed 2026-05-17. The retired Rust `crates/ark-lexer` crate was removed from
the workspace. Current lexer source-of-truth is selfhost
`src/compiler/lexer.ark`.

## Acceptance

- [x] `crates/ark-lexer/` directory removed (`[ ! -d crates/ark-lexer ]`)
- [x] Workspace `Cargo.toml` `members` array no longer lists `crates/ark-lexer`
- [x] No other crate's `Cargo.toml` lists `ark-lexer` as a dependency
- [x] `Cargo.lock` regenerated and contains no `name = "ark-lexer"`
- [x] No source / script / docs reference remains for `ark_lexer`, `ark-lexer`, or `crates/ark-lexer`
- [x] `python scripts/manager.py verify` passes
- [x] 4 canonical selfhost gates pass with FAIL=0 and no SKIP increase

## Close Notes

- Removed `crates/ark-lexer/`.
- Removed `crates/ark-lexer` from `Cargo.toml` workspace members,
  default-members, and workspace dependencies.
- Regenerated `Cargo.lock`.
- Updated current lexer docs to reference selfhost `src/compiler/lexer.ark`.

## Verification Results

- `python scripts/manager.py verify`: PASS (23 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixpoint`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost fixture-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost parity --mode --cli`: PASS (1 passed, 0 skipped, 0 failed)
- `python scripts/manager.py selfhost diag-parity`: PASS (1 passed, 0 skipped, 0 failed)
- `cargo check --workspace`: PASS
- `rg -l "\bark_lexer\b|\bark-lexer\b|crates/ark-lexer|name = \"ark-lexer\"" Cargo.toml Cargo.lock crates/ scripts/ src/ docs/ .github/`: no files
- `test ! -d crates/ark-lexer`: PASS
- `grep -F "crates/ark-lexer" Cargo.toml`: no output
- `grep -RIn "\bark-lexer\b" crates/*/Cargo.toml`: no output

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
