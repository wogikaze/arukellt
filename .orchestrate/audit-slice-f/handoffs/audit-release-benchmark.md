<!-- orchestrate handoff
task: audit-release-benchmark
branch: master
-->

## Status
success

## Branch
`master`

## What I did
- Reviewed 57 Slice F release/benchmark/CI issues against `scripts/manager.py`, `docs/release-criteria.md`, `benchmarks/`, and `mise.toml`.
- Reopened #418 and #422 (missing hygiene scripts cited in close evidence).
- Added audit-resolution notes to #109, #140, #146, #149, #531, #547 for stale 2026-04-03 reopen metadata.
- Appended Slice F wave to `docs/process/false-done-audit-2026-06-12.md`.
- Regenerated `issues/open/index.md` and dependency graph.

## Verification
type-check-only

`python3 scripts/manager.py verify quick` → 143/149 pass (6 pre-existing failures unchanged).

## Notes, concerns, deviations, findings, thoughts, feedback
- Cloud worker spawn unavailable (`CURSOR_API_KEY` unset); subplanner executed audit directly.
- Release issues #546–556 retained done based on 2026-05-17 recheck evidence.
- #531 epic status corrected to done (all #532–537 children closed).

## Suggested follow-ups
- Implement `check-orphan-inventory.sh` or manager.py gate for #418/#422.
- Monitor #544 bench directory layout vs stated subdir goals.
