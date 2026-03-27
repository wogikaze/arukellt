# T3 perf and size telemetry

**Status**: open
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 013
**Depends on**: 002, 004, 005, 006, 007, 008, 009, 010
**Track**: parallel
**Blocks v1 exit**: no

## Summary
Add dedicated T3 compile-time, binary-size, and runtime telemetry so WasmGC completion does not hide severe regressions.

## Acceptance Criteria
- [ ] T3 compile-time baselines exist for representative sources.
- [ ] T3 binary-size telemetry is recorded and comparable over time.
- [ ] Representative T1/T3 runtime comparisons are available without destabilizing normal CI.
- [ ] Telemetry output is machine-readable and version-controlled where policy requires.

## Goal
Watch the cost of the T3 transition while keeping normal CI deterministic.

## Implementation
- Extend or split benchmark scripts to collect T3-specific compile, size, and runtime telemetry.
- Record T3 metrics in `tests/baselines/` or the benchmark results location used by current policy.
- Add telemetry fields for GC-specific backend characteristics where useful (heap type counts, fallback/bridge usage counts, etc.).
- Keep heavy comparison jobs separate from default correctness verification.

## Dependencies
- Issues 002 and 004+, plus target/runtime stabilization.

## Impact
- `scripts/collect-baseline.py`
- benchmark scripts
- baseline artifacts
- benchmark docs

## Tests
- Telemetry schema tests.
- Benchmark script smoke tests.

## Docs updates
- `docs/process/benchmark-results.md`
- `docs/contributing.md`

## Compatibility
- No user-facing behavior changes.

## Notes
- This is a parallel track; it should not block correctness work from proceeding.
