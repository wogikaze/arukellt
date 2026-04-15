# CLI --wit flag, --emit component workflow, docs

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-15
**ID**: 034
**Depends on**: 030, 031, 028b
**Track**: component-model
**Blocks v1 exit**: no

## Completion note — 2026-04-15

This issue was reopened by audit after the CLI accepted `--wit` but still failed
to compile source that actually called WIT-imported functions. The remaining gap is
now closed.

## Summary

Complete the end-to-end CLI workflow for Component Model usage. Ensure `--wit <path>`
is threaded into the real component compile path, preserve that wiring for
`--emit all`, update the current docs to match the real command surface, and keep
the workflow covered by regression tests.

## What landed

- `Session` now registers callable WIT import signatures before type-check, so
  source code can call WIT-imported functions during component-oriented builds.
- `Session` continues to populate `MirModule.imports` from the same WIT inputs.
- `arukellt compile --emit all ... --wit ...` now preserves `wit_files` and the
  same session options for the component build instead of silently dropping them.
- CLI-facing regression coverage exists in `crates/arukellt/tests/component_cli.rs`.
- Driver-level regression coverage exists in
  `crates/ark-driver/tests/wit_import_roundtrip.rs` for a real imported call.
- `docs/current-state.md` and `docs/quickstart.md` now describe the current
  component / WIT command surface accurately.

## Acceptance Criteria

- [x] `--wit <path>` is accepted by the `compile` subcommand.
- [x] WIT-imported functions compile through the real frontend/typecheck path.
- [x] `--emit component` remains wired on `wasm32-wasi-p2`.
- [x] `--emit all` preserves WIT import wiring for the component branch.
- [x] Current docs reflect the real command surface.
- [x] At least one CLI-facing regression test or fixture covers the path.

## Verification

- [x] `cargo test -p ark-driver --test wit_import_roundtrip`
- [x] `cargo test -p arukellt --test component_cli`
- [x] `cargo build --workspace --exclude ark-llvm`
- [x] `python3 scripts/check/check-docs-consistency.py`
- [x] `bash scripts/run/verify-harness.sh --quick`

## Notes

- Component wrapping still requires external `wasm-tools`, so the CLI regression
  test skips cleanly when that dependency is unavailable in the local environment.
- The component target remains `wasm32-wasi-p2`; docs now state that explicitly.