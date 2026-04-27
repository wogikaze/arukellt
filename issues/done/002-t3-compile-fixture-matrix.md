---
Status: done
Created: 2026-03-27
Updated: 2026-03-30
ID: 2
Track: main
Depends on: 1
Orchestration class: implementation-ready
---
# T3 compile fixture matrix
**Blocks v1 exit**: yes

## Summary

Create a dedicated T3 compile verification matrix so `wasm32-wasi-p2` compile completeness is measured directly rather than inferred from T1 or mixed end-to-end tests.

## Acceptance Criteria

- [x] Fixture harness can run T3 compile-only cases independently from T1 run cases.
- [x] T3 compile fixtures cover modules, traits, methods, operators, generics, `?`, loops, strings, vecs, closures, structs, enums, and match.
- [x] Representative large samples such as `docs/sample/parser.ark` are included in the T3 compile smoke path.
- [x] Baseline output clearly separates compile failures from runtime failures for T3.

## Goal

Make T3 compile correctness observable and enforceable.

## Implementation

- Extend `crates/arukellt/tests/harness.rs` and manifest parsing to support a T3 compile-only fixture kind (for example `t3-compile:`) without changing public CLI.
- Add or reorganize fixtures under `tests/fixtures/` so T3 compile coverage exists for:
  - modules/imports
  - trait/impl/method/operator overload
  - nested generics
  - `?`
  - `for range`, `for values`, iterator-style `for`
  - String and interpolation
  - Vec operations
  - closures
  - structs/enums/match
- Add a T3 compile smoke path for `docs/sample/parser.ark` and any other branch-standard large sample.
- Extend baseline collection so T3 compile pass/fail and primary diagnostics can be diffed over time.

## Dependencies

- Issue 001 must define T3 exit criteria first.

## Impact

- Test tree, baseline files, verify scripts, and fixture manifest all change.

## Tests

- Harness unit/integration tests.
- Full manifest self-check.
- T3 compile smoke for representative large sources.

## Docs updates

- `docs/contributing.md`
- `docs/current-state.md`
- `docs/process/policy.md`

## Compatibility

- No public CLI changes.
- Verification becomes stricter because T3 compile failures are now first-class regressions.

## Notes

- Keep compile-only and runtime failures distinct so the T3 emitter can be completed incrementally without masking defects.