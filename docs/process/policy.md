# Operational Policy

> **Current-first**: this page describes the current operational contract, not aspirational design docs.
> Target names follow ADR-007 / ADR-013. Structured status lives in [`../data/project-state.toml`](../data/project-state.toml).

## Capability Defaults

| Capability | Default access | Deny mechanism | Enforcement phase | Transitive |
|------------|----------------|----------------|-------------------|------------|
| Standard I/O (stdin/stdout/stderr) | Allow | Cannot be denied | — | — |
| Filesystem access | Deny unless granted | `--dir path:ro\|rw`, `--deny-fs` | Runtime grant / compile policy | n/a |
| Clock/time | Allow | `--deny-clock` (intended) | **Unimplemented** on selfhost CLI; intended: compile-time MIR scan on `run` | Yes (intended) |
| Random numbers | Allow | `--deny-random` (intended) | **Unimplemented** on selfhost CLI; intended: compile-time MIR scan on `run` | Yes (intended) |

Notes:

- **Intended contract** (Rust-era #291): `--deny-clock` / `--deny-random` hard-error at
  compile time via transitive MIR scan (`mir_uses_capability`) on `arukellt run`
  (not on `compile`).
- **Current selfhost reality**: those flags are **not** accepted by the selfhost CLI
  (`src/compiler/main/args_parse*.ark` has no `--deny-clock` / `--deny-random`).
  Related fixtures (`deny_clock_compile.ark`, `deny_random_compile.ark`,
  `stdlib_io/deny_clock.ark`, `stdlib_io/deny_random.ark`) are in
  `DIAG_PARITY_SKIP` (#459). Do **not** claim they are implemented until the flags
  exist and parity skips are removed.
- Filesystem remains deny-by-default without `--dir`.
- Structured SSOT: [`../data/capabilities.toml`](../data/capabilities.toml).

## Generated Artifacts

| Artifact | Commit? | Regeneration |
|----------|---------|--------------|
| Baseline JSON under `tests/baselines/` | Yes | `python3 scripts/util/collect-baseline.py` |
| Generated docs / landing pages / status blocks | Yes | `python3 scripts/gen/generate-docs.py` |
| Issue queue indexes under `issues/open/` | Yes | `python3 scripts/gen/generate-issue-index.py` |
| CLI help text | No | built from source |

## Target Support Policy

Canonical public targets (see `docs/current-state.md` / `project-state.toml`):

| Target | Support tier | Implementation | Notes |
|--------|--------------|----------------|-------|
| `wasm32-gc` | primary | partial | CLI default; Wasm GC + WASI P2 host profile (ADR-013) |
| `wasm32` | supported | complete | Linear-memory / AtCoder compatibility path |
| `native-cpp` | scaffold | scaffold | C99 emit scaffold; not runnable |
| `native-llvm` | scaffold | scaffold | LLVM IR scaffold。ADR-049はnative-cppだけを決定し、本targetのABIは未決定 |

Retired / not public:

| Name | Policy |
|------|--------|
| `wasm32-freestanding` | Hard error as public target (ADR-007) |
| Legacy aliases (`wasm32-wasi-p1`, `wasm32-wasi-p2`, …) | Input compatibility only (`W0002`); never write as current IDs |

Host profiles (`wasi-p1` / `wasi-p2` / `wasi-p3`) are **not** language targets. See `[[host_profiles]]` in `project-state.toml`.

## Emit Policy

Separate **public contract** from **living implementation**:

| Axis | Current statement |
|------|-------------------|
| Public contract (ADR-008) | `--emit component` / `--emit wit` / `--emit all` on `wasm32-gc` are in-tree compiler responsibilities |
| Implementation state | Living path may still invoke `wasm-tools` / Python wrap helpers for some component packaging steps (see `current-state.md` ADR gaps) |
| External requirements | Do not assume a clean environment without `wasm-tools` until the in-tree path is complete (#714 / related) |
| Default emit | `--emit core-wasm` |

- Component export support covers the currently wired WIT-compatible surface; not every canonical ABI case is complete.
- `--emit component` is **not** part of a historical v1 completion gate; treat tier as smoke until release criteria say otherwise.

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

Normal correctness verification lives in `scripts/manager.py` and should stay deterministic.

The default invocation is the fast local gate. It covers:

- docs structure checks
- docs consistency drift checks
- manifest completeness / registration checks
- stdlib manifest checks
- cheap deterministic policy checks

Heavier groups are explicit and opt-in locally:

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
- See `docs/process/benchmark-plan.md` and `docs/benchmarks/governance.md`.
- Do not publish all-skipped or deprecated-target runs as current performance evidence.

## Hidden Developer Support

These are development aids, not stable public CLI surface.

- `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`
- `ARUKELLT_DUMP_DIAGNOSTICS=1`

## Historical Gates vs Current Reality

- Early completion gates centered on primary-target core-wasm compile/run correctness (historically labeled T3 / `wasm32-wasi-p2`).
- Component Model support was added on top of that base.
- Old docs that still describe `--emit component` as a hard error, or that treat T1 as the production path, are historical — prefer `docs/current-state.md`.

## Compatibility / Migration Policy

- Current behavior source of truth is `docs/current-state.md` plus executable baselines.
- Internal API migration docs should distinguish old Session API vs newer artifact / query-oriented surfaces.
- Docs must not drift on target status, verification policy, diagnostic severity, or component support status.
