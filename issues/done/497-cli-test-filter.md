---
Status: done
Created: 2026-04-14
Updated: 2026-04-14
Track: cli
Source: "audit — extensions/arukellt-all-in-one/src/extension.js:740"
Orchestration class: implementation-ready
Depends on: none
# 497 — CLI test runner: add --filter for single-test execution
---
# 497 — CLI test runner: add --filter for single-test execution

## Summary

The `arukellt test` command does not support `--filter` for running a single
test function. The VS Code extension currently runs all tests in the file
because this flag is missing. The extension code explicitly notes this gap.

## Primary paths

- `crates/arukellt/src/`
- `extensions/arukellt-all-in-one/src/extension.js`

## Non-goals

- Test discovery protocol (separate concern)
- Test framework design changes

## Acceptance

- [x] `arukellt test --filter <name>` runs only matching test functions
- [x] Extension CodeLens "Run Test" uses `--filter` when available
- [x] At least one integration test exercises `--filter`
- [x] `cargo test` passes

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes
- `arukellt test --filter` does not error on unknown flag

## Close gate

Acceptance items checked; extension.js updated to remove "does not yet support --filter" note.