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
| Generated docs / landing pages / status blocks | Yes | `python3 scripts/generate-docs.py` |
| Issue queue indexes under `issues/open/` | Yes | `bash scripts/generate-issue-index.sh` |
| CLI help text | No | built from source |

## Target Support Policy

| Target | Status | Support level |
|--------|--------|---------------|
| `wasm32-wasi-p1` (T1) | Implemented | Compatibility path for non-GC environments |
| `wasm32-freestanding` (T2) | Registry only | Not implemented |
| `wasm32-wasi-p2` (T3) | Implemented | Canonical GC-native path |
| `native` (T4) | LLVM scaffold only | Not implemented |
| `wasm32-wasi-p3` (T5) | Future | Not implemented |

## Emit Policy

- `--emit core-wasm` is the default production artifact path.
- `--emit component`, `--emit wit`, and `--emit all` are available on `wasm32-wasi-p2`.
- Component output currently depends on an external `wasm-tools` binary and a WASI adapter module.
- Component export support is implemented for the currently wired WIT-compatible surface; not every canonical ABI case is complete yet.
- `--emit component` is implemented in v2, but it is **not** part of the original v1 completion gate.

## Diagnostic Quality Standards

- Every diagnostic has a canonical code.
- Registry tracks severity and phase origin.
- Type mismatch diagnostics should expose expected / actual when possible.
- Fix hints should be rendered when the compiler has one.
- Deterministic structured snapshots are available for tests / docs.

### Canonical current warnings/errors

- `W0001`: warning, `typecheck`, same-body shared alias heuristic
- `W0002`: warning, `target`, deprecated target alias
- `W0004`: error, `backend-validate`, generated Wasm failed validation
- `W0005`: warning, `component`, non-exportable function skipped from component exports

## Verification Policy

Normal correctness verification lives in `scripts/verify-harness.sh` and should stay deterministic.

The default invocation is the fast local gate. It covers:

- docs structure checks
- docs consistency drift checks
- manifest completeness / registration checks
- stdlib manifest checks
- cheap deterministic policy checks

Heavier groups are explicit and opt-in locally:

- `--cargo` for fmt / clippy / workspace tests
- `--fixtures` for the manifest-driven fixture harness
- `--baseline` for baseline collection smoke
- `--size` for the hello.wasm size gate
- `--wat` for WAT roundtrip verification
- `--full` for the full local verification set

CI should request the heavy groups explicitly rather than relying on the fast default mode.
The current gate totals belong to `docs/current-state.md` and the harness output; do not hardcode them here unless policy itself changes.

## Perf Policy

- Baseline sources are frozen under `tests/baselines/`.
- `arukellt check` median compile-time budget: within 10% of baseline.
- `arukellt compile` median compile-time budget: within 20% of baseline.
- Heavy perf comparisons are split from the normal correctness gate to avoid unstable CI.

## Hidden Developer Support

These are development aids, not stable public CLI surface.

- `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`
- `ARUKELLT_DUMP_DIAGNOSTICS=1`

## Historical Gates vs Current Reality

- v1 completion was defined around **T3 core-wasm compile/run correctness**.
- v2 added Component Model support on top of that base.
- Old docs that still describe `--emit component` as a hard error should be treated as historical and updated or ignored in favor of `docs/current-state.md`.

## Compatibility / Migration Policy

- Current behavior source of truth is `docs/current-state.md` plus executable baselines.
- Internal API migration docs should distinguish old Session API vs newer artifact / query-oriented surfaces.
- Docs must not drift on target status, verification policy, diagnostic severity, or component support status.
