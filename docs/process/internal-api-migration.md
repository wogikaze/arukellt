# Internal API Migration Notes

> Internal consumer guidance for the pipeline refactor.
> Current source of truth remains the code plus `docs/current-state.md`.

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
