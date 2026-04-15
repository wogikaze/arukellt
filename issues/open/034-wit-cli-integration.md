# CLI --wit flag, --emit component workflow, docs

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-13
**ID**: 034
**Depends on**: 030, 031, 028b
**Track**: component-model
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: CLI accepts --wit flag but only validates file existence. Not threaded into resolver/session/component compile.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Parent note — 2026-04-15

The remaining `--wit` pipeline gap is tracked concretely in [#028b](../done/028b-wit-import-pipeline-wiring.md).
Treat this issue as blocked on #028b for the import-binding path; other CLI/docs claims still
require evidence review before close.

## Summary

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
