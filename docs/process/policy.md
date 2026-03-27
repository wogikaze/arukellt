# Operational Policy

> **Current-first**: this page describes the current operational contract, not aspirational design docs.

## Capability Defaults

| Capability | Default | Override |
|------------|---------|----------|
| Standard I/O (stdin/stdout/stderr) | Allow | Cannot be denied |
| Filesystem access | Deny by default unless granted | `--dir path:ro`, `--dir path:rw`, `--deny-fs` |
| Clock/time | Runtime provides access; `--deny-clock` is a hard error | future enforcement |
| Random numbers | Runtime provides access; `--deny-random` is a hard error | future enforcement |

## Generated Artifacts

| Artifact | Commit? | Regeneration |
|----------|---------|--------------|
| Baseline JSON under `tests/baselines/` | Yes | `python3 scripts/collect-baseline.py` |
| Target reference / docs sync material | Yes | via docs + consistency checks |
| CLI help text | No | built from source |

## Target Support Policy

| Target | Status | Support level |
|--------|--------|---------------|
| `wasm32-wasi-p1` (T1) | Implemented | Full correctness gate, default path |
| `wasm32-freestanding` (T2) | Planned | Registry only |
| `wasm32-wasi-p2` (T3) | Experimental fallback | Uses T1 runtime path internally |
| `native` (T4) | Planned | Optional/scaffold only |
| `wasm32-wasi-p3` (T5) | Future | Registry only |

## Emit Policy

- `--emit component` is currently a hard error
- `--emit all` is currently blocked for the same reason
- current production artifact is core Wasm
- Component/WIT docs are design or migration context, not current shipped behavior

## Diagnostic Quality Standards

- Every diagnostic has a canonical code
- Registry tracks severity and phase origin
- Type mismatch style diagnostics should expose expected/actual when possible
- Fix hints should be rendered when the compiler has one
- Deterministic structured snapshots are available for tests/docs

### Canonical current warnings/errors

- `W0001`: warning, `typecheck`, same-body shared alias heuristic
- `W0002`: warning, `target`, deprecated target alias
- `W0004`: error, `backend-validate`, generated Wasm failed validation

## Verification Policy

Normal correctness verification lives in `scripts/verify-harness.sh` and should stay deterministic.

It includes:

- docs structure checks
- docs consistency drift checks
- fmt / clippy / build / tests
- manifest-driven fixture harness
- stdlib manifest check
- baseline collection smoke

## Perf Policy

- Baseline sources are frozen under `tests/baselines/`
- `arukellt check` median compile time budget: within 10% of baseline
- `arukellt compile` median compile time budget: within 20% of baseline
- Heavy perf comparisons are split from the normal correctness gate to avoid unstable CI

## Hidden Developer Support

These are development aids, not stable public CLI surface.

- `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`
- `ARUKELLT_DUMP_DIAGNOSTICS=1`

## V1 Completion Gate

V1 exit is defined by T3 (`wasm32-wasi-p2`) core-wasm compile/run correctness with WasmGC-native data representations. The canonical criteria are documented in `docs/current-state.md` § V1 Exit Criteria.

Operational requirements for the gate:

- All T3 compile fixtures pass without fallback to T1 linear-memory runtime.
- `RuntimeModel::T3FallbackToT1` is replaced by a non-fallback model.
- `verify-harness.sh` passes with T3 compile verification enabled.
- `--emit component` is **not** part of the v1 gate and remains a hard error.
- T4 (native/LLVM) is **not** part of the v1 gate.

## Compatibility / Migration Policy

- current source of truth is `docs/current-state.md` + executable baselines
- internal API migration must document old Session API → new artifact API mapping
- intentional behavior change in this track: `W0004` is now a hard error
- docs must not drift on fixture count, target status, component emit status, or warning/error severity
