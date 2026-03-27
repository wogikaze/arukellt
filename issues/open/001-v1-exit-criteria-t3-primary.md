# V1 exit criteria: T3 primary path

**Status**: open
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 001
**Depends on**: none
**Track**: main
**Blocks v1 exit**: yes

## Summary

Fix the branch-wide definition of v1 completion around `--target wasm32-wasi-p2` as the canonical path, with WasmGC-native compile/run completed and T1 retained only as a compatibility path.

## Acceptance Criteria

- [ ] `docs/current-state.md` defines v1 completion in terms of T3 compile/run correctness, WasmGC-native data model completion, and fallback removal.
- [ ] `docs/process/policy.md` states that `--emit component` is not required for v1 exit and remains out of scope for this milestone.
- [ ] The exit criteria distinguish current shipped behavior from target ideal behavior so future docs do not regress into ambiguity.
- [ ] Every later T3 issue in `./issues/open` can reference this issue as the canonical completion gate.

## Goal

Define a non-ambiguous completion contract for v1 so that implementation, tests, CI, and docs converge on the same target: `wasm32-wasi-p2` becomes the primary path when it is no longer a T1/P1 fallback and its WasmGC compile/run path is real.

## Implementation

- Update `docs/current-state.md` to add a dedicated v1 exit section.
- Update `docs/process/policy.md` to record the operational gate for T3 completion.
- Update `docs/migration/t1-to-t3.md` so that the migration document explicitly states the v1 core exit is T3 core-wasm compile/run completion, not component-model completion.
- Add a short reference to the exit criteria in `docs/process/v1-status.md` or equivalent status page so the branch status page and current-state page cannot diverge.

## Dependencies

- None. This must land first because all later issues use this as their completion target.

## Impact

- Documentation only, but it changes how success/failure is judged across the branch.
- Affects CI expectations, docs wording, and release status language.

## Tests

- Run docs consistency/link checks after edits.
- Verify that no current-first doc still implies component emit is part of v1 exit.

## Docs updates

- `docs/current-state.md`
- `docs/process/policy.md`
- `docs/migration/t1-to-t3.md`
- status page (`docs/process/v1-status.md` or current status equivalent)

## Compatibility

- No runtime/compiler behavior change.
- Changes the official interpretation of “done” only.

## Notes

- Adopt current-first pages over aspirational ADR wording when the two disagree.
- Explicitly state that T3 must stop being a fallback before v1 can be called complete.
