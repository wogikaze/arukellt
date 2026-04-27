---
Status: open
Created: 2026-03-28
Updated: 2026-04-22
ID: 28
Track: component-model
Depends on: 030, 031, 028b, 616
Orchestration class: blocked-by-upstream
Orchestration upstream: None
Blocks v{N}: none
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Reason: CLI accepts --wit flag but only validates file existence. Not threaded into resolver/session/component compile.
Action: Moved from `issues/done/` to `issues/open/` by false-done audit.
BLOCKED: "This issue is structurally blocked by [#616](616-selfhost-component-emit-infra.md) (Component emission infrastructure missing in the newly unified selfhost compiler). Do not dispatch until the emitter logic is capable of generating Wasm components."
---

- The `--wit` flag threading: "CLI → Session → Resolver (for import binding) → MIR"
"error: component model requires --target wasm32-wasi-p2"
- `cmd_compile` forwards only `target`, `opt_level`, and `emit_mode` into `driver: ":compile_file`; no WIT import list reaches the resolver/session/component pipeline."
- `driver: ":compile_source` only recognizes `emit_mode` values `wat` and `wasm`; there is no component emit branch or `.component.wasm` output path."
- Missing `wasm-tools`: ""error: wasm-tools not found. Install with: cargo install wasm-tools""
- Non-exportable function: ""warning W0005: function `foo` has closure parameter, skipped from component exports""
- WIT parse error: ""error: host.wit:3:5: expected type name, found `{`""
- [x] Target help text updated: `wasm32-wasi-p2` description changed from
# CLI --wit flag, --emit component workflow, docs

Reason: CLI accepts --wit flag but only validates file existence. Not threaded into resolver/session/component compile.
Action: Moved from `issues/done/` to `issues/open/` by false-done audit.
- The `--wit` flag threading: "CLI → Session → Resolver (for import binding) → MIR"
"error: component model requires --target wasm32-wasi-p2"
---
# CLI --wit flag, --emit component workflow, docs

## Reopened by audit — 2026-04-13



## Parent note — 2026-04-15

The remaining `--wit` pipeline gap is tracked concretely in [#028b](../done/028b-wit-import-pipeline-wiring.md).
Treat this issue as blocked on #028b for the import-binding path; other CLI/docs claims still
require evidence review before close.

## Evidence review — 2026-04-22

Checked the current selfhost sources in `src/compiler/main.ark` and `src/compiler/driver.ark`.
The reopened acceptance is not yet satisfied in the selfhost-target repo state:

- `parse_args` accepts `--target`, `--opt-level`, `--emit`, `-o`, `--dump-phases`, `--help`, `--json`, and `--version`, but there is no `--wit` flag parsing or storage.
- `cmd_compile` forwards only `target`, `opt_level`, and `emit_mode` into `driver::compile_file`; no WIT import list reaches the resolver/session/component pipeline.
- `driver::compile_source` only recognizes `emit_mode` values `wat` and `wasm`; there is no component emit branch or `.component.wasm` output path.

The docs under `docs/current-state.md`, `docs/migration/v1-to-v2.md`, and `docs/platform/abi.md` still describe component support, but the selfhost CLI/component implementation evidence does not match those claims yet. Leave this issue open until the actual CLI threading and component workflow are present in `src/compiler/`.

## Partial slice — 2026-04-22

Implemented only the selfhost CLI parsing/storage surface for repeated `--wit <path>` flags.
The change keeps `--wit` paths on the CLI/config objects and leaves resolver binding,
component emission, and all other component-model pipeline work untouched.
This issue remains open for the remaining pipeline slice.

Implemented the selfhost emit-mode acceptance slice for `--emit component`.
The CLI now short-circuits `component` with a targeted not-yet-wired error,
and the driver keeps an explicit `component` fallback instead of falling
through as a generic unsupported emit mode. Actual component generation and
WIT resolver binding remain out of scope.

## Summary

**BLOCKED:** This issue is structurally blocked by [#616](616-selfhost-component-emit-infra.md) (Component emission infrastructure missing in the newly unified selfhost compiler). Do not dispatch until the emitter logic is capable of generating Wasm components.

Complete the end-to-end CLI workflow for Component Model usage. Add `--wit <path>`
flag for import binding, update `--emit component` to produce `.component.wasm`,
update all relevant documentation, and write the v1→v2 migration guide.

## Context

The CLI (`crates/arukellt/src/main.rs`) currently supports `--emit core-wasm` and
`--emit wit`. The `--emit component` path hits a hard error. The `--wit` flag for
specifying import WIT files does not exist.

The user-facing workflow for v2 component usage should be:

```bash
# Generate WIT from Arukellt source (existing, works)
arukellt compile --emit wit mylib.ark

# Compile to component (new)
arukellt compile --emit component mylib.ark --target wasm32-wasi-p2

# Compile with host imports (new)
arukellt compile --emit component myapp.ark --wit host.wit --target wasm32-wasi-p2
```

## Acceptance Criteria

- [x] `--wit <path>` CLI flag added to the `compile` subcommand. Accepts a path to a
      `.wit` file. Multiple `--wit` flags are accepted for multiple interface files.
- [x] `--emit component` produces `<name>.component.wasm` alongside or instead of
      `<name>.wasm` (configurable via `--emit all` for both).
- [x] `--emit all` is unblocked and produces both core + component output.
- [x] Error messages for component-related failures are clear and actionable:
      - Missing `wasm-tools`: "error: wasm-tools not found. Install with: cargo install wasm-tools"
      - Non-exportable function: "warning W0005: function `foo` has closure parameter, skipped from component exports"
      - WIT parse error: "error: host.wit:3:5: expected type name, found `{`"
- [x] `docs/current-state.md` updated:
      - V2 exit status section added
      - `--emit component` status changed from "hard error" to "implemented"
      - Component model test count added to test health
- [x] `docs/migration/v1-to-v2.md` written with:
      - Breaking changes (if any)
      - New CLI flags (`--wit`, `--emit component`)
      - How to create a component from existing Arukellt code
      - Known limitations of v2 component support
- [x] `docs/platform/abi.md` updated with Layer 2B (canonical ABI) documentation:
      - GC ref ↔ canonical ABI conversion rules
      - Linear memory budget for canonical ABI (64KB - 256 = 65280 bytes)
      - Import/export conventions
- [x] `docs/stdlib/core.md` updated if any stdlib functions are affected by component
      boundaries (e.g., I/O functions in component mode).
- [x] Target help text updated: `wasm32-wasi-p2` description changed from
      "component model not yet implemented" to "component model supported".

## Key Files

- `crates/arukellt/src/main.rs` — CLI flag additions
- `crates/ark-driver/src/session.rs` — `compile_component()` with `--wit` support
- `crates/ark-target/src/lib.rs` — update T3 help text
- `docs/current-state.md` — v2 status
- `docs/migration/v1-to-v2.md` — new file
- `docs/platform/abi.md` — Layer 2B documentation

## Notes

- The `--wit` flag threading: CLI → Session → Resolver (for import binding) → MIR
  (for import entries) → Backend (for import section generation).
- Default target for `--emit component` should be `wasm32-wasi-p2` (not `wasm32-wasi-p1`).
  If user specifies T1 with `--emit component`, emit a clear error:
  "error: component model requires --target wasm32-wasi-p2"
- Consider adding `arukellt component` as a future subcommand alias (not for v2).