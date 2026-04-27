---
Status: Done
Track: main
Orchestration class: implementation-ready
Depends on: none
---

(fields: `lex_ms`, `parse_ms`, `resolve_ms`, `typecheck_ms`, `lower_ms`,
# Issue #141 — Add cold/warm/incremental compile latency breakdown to benchmark harness

## Summary

Add per-compiler-phase timing to the benchmark output so operators can see where
compile time goes (lex, parse, resolve, typecheck, lower/MIR, opt, emit) rather
than just total wall-clock.

## Acceptance Criteria

- [x] Benchmark output includes `compile.phase_ms` object with keys:
      `lex`, `parse`, `resolve`, `typecheck`, `lower`, `opt`, `emit`, `total`
- [x] `benchmarks/schema.json` defines `phase_ms` under `compile_metrics`
- [x] Phase data is reported per benchmark in the JSON result file
- [x] Human-readable phase summary is printed to the terminal during a run
- [x] Gracefully skipped (field is `null`) when compiler is unavailable or
      running in selfhost mode
- [x] `bash scripts/run/verify-harness.sh --quick` exits 0

## Implementation Notes

- Uses `arukellt compile --json` to get machine-readable `CompileTiming` output
  (fields: `lex_ms`, `parse_ms`, `resolve_ms`, `typecheck_ms`, `lower_ms`,
  `opt_ms`, `emit_ms`, `total_ms`).
- One phase-timing pass per benchmark (separate from the main compile loop so
  that iteration count and wasm output are unaffected).
- `scripts/run/run-benchmarks.sh` collects `PHASE_MS_JSON` and injects it into
  the `compile` object in the result JSON.
- Schema field `phase_ms` was added to `benchmarks/schema.json` under the
  `compile_metrics` definition.

## Files Changed

- `scripts/run/run-benchmarks.sh` — phase timing collection and JSON output
- `benchmarks/schema.json` — `phase_ms` field definition (already present)