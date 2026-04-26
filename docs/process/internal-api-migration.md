# Internal API Migration Notes

> Internal consumer guidance for the T3 promotion and pipeline refactor.
> Current source of truth remains the code plus `docs/current-state.md`.

## T1→T3 Internal API Changes

| Old assumption | New reality | Notes |
|---|---|---|
| `RuntimeModel::T3FallbackToT1` | Removed | T3 no longer falls back to T1 |
| `RuntimeModel::T3WasmGcP2` | Active T3 model | Sole runtime model for `wasm32-wasi-p2` |
| T3 `experimental: true` | `experimental: false` | T3 is canonical v1 path |
| T1 as primary compile path | T1 as compatibility path | T3 compiles all fixture categories |
| `emit_with_plan` T3 fallback arm | Single `T3WasmGcP2` arm | No fallback dispatch |

## Intentional behavior change

- `W0004` is now a backend validation hard error

## Session API direction

| Old shape | New / target shape | Notes |
|---|---|---|
| `Session::check(path)` | `Session::parse/bind/load/analyze/resolve/check_core_hir` | `check` should stop before backend work |
| `Session::compile(path, target)` | `Session::lower_mir/opt_mir/plan_backend/emit_wasm` | full path only for compile/run |
| direct `resolve → typecheck → mir → wasm` chaining | Session-owned artifact/query flow | keep phase control centralized in Session |
| ad-hoc warning rendering | canonical diagnostics registry | code/severity/phase must come from `ark-diagnostics` |

## Compatibility guidance

- keep shim paths while internal consumers migrate
- document old entrypoint → new artifact boundary mapping
- do not expose hidden dump tooling as stable CLI

## Hidden developer tooling

- `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`
- `ARUKELLT_DUMP_DIAGNOSTICS=1`
