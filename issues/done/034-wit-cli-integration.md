---
Status: done
Created: 2026-03-28
Updated: 2026-06-12
ID: 28
Track: component-model
Depends on: 030, 031, 028b, 124
Orchestration class: blocked-by-upstream
Orchestration upstream: 124
Blocks v{N}: none
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Reason: CLI accepts --wit flag but only validates file existence. Not threaded into resolver/session/component compile.
Action: Moved from `issues/done/` to `issues/open/` by false-done audit.
Blocked by: "#124 WIT component import syntax"
---

## Audit resolution — 2026-06-12

Callable WIT import binding is now proven in-repo (supersedes the 2026-05-15
"Still open" note):

- `tests/fixtures/wit_import/main.ark` — `import "test:calculator/math"` with
  `math::add` / `math::multiply` calls; registered as
  `component-compile:wit_import/main.ark` in `tests/fixtures/manifest.txt`
- `tests/fixtures/component/import_scalar_func.ark` — imports `test:host/math`
  and calls `math::add(1, 2)`; registered as `component-compile:component/import_scalar_func.ark`
- Selfhost compile of both fixtures with `--emit component --wit ...` succeeds
  (phase 6) on the current stage-3 compiler

**Classification:** `truly-done` for WIT CLI + import binding acceptance.
Stale reopen / "moved to open" metadata from 2026-04 audits is historical only.

---

- The `--wit` flag threading: "CLI → Session → Resolver (for import binding) → MIR"
"error: component model requires --target wasm32-wasi-p2"
- `cmd_compile` forwards only `target`, `opt_level`, and `emit_mode` into `driver: ":compile_file`; no WIT import list reaches the resolver/session/component pipeline."
- `driver: ":compile_source` only recognizes `emit_mode` values `wat` and `wasm`; there is no component emit branch or `.component.wasm` output path."
- Missing `wasm-tools`: ""error: wasm-tools not found. Install with: cargo install wasm-tools""
- Non-exportable function: ""warning W0005: function `foo` has closure parameter, skipped from component exports""
- WIT parse error: ""error: host.wit:3:5: expected type name, found `{`""
- [x] Target help text updated: `wasm32-wasi-p2` description changed fro

CLI --wit flag, --emit component workflow, docs

Reason: CLI accepts --wit flag but only validates file existence. Not threaded into resolver/session/component compile.
Action: Moved from `issues/done/` to `issues/open/` by false-done audit.
- The `--wit` flag threading: "CLI → Session → Resolver (for import binding) → MIR"
"error: component model requires --target wasm32-wasi-p2"
--

CLI --wit flag, --emit component workflow, docs

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

## Recheck — 2026-05-14

- **Close candidate:** No.
- **Current selfhost source:** `src/compiler/main.ark` does not currently retain
  `--wit` paths in `CliOptions`, and `cmd_compile` forwards only `target`,
  `opt_level`, and `emit_mode` to `driver::compile_file`. No WIT import list
  reaches resolver/typecheck/MIR.
- **Emit-mode source state:** `src/compiler/driver.ark` contains a current-source
  `emit_mode == "component"` branch that calls `component_emitter::emit_component`,
  but there is no `--emit all` output workflow in `cmd_compile`.
- **Active CLI state:** `.build/selfhost/arukellt-s2.wasm` is absent, so the Rust
  shell uses `bootstrap/arukellt-selfhost.wasm`. That pinned compiler still
  rejects `--emit component` with `error[E0500|emit]: unsupported emit mode:
  component`.
- **Verification evidence:** `python3 scripts/manager.py verify component` fails
  before interop invocation tests because component emission is unavailable on
  the active command path.
- **Follow-up needed before closure:** restore/refresh the current selfhost wasm,
  then implement and test `--wit` threading plus `--emit component` / `--emit all`
  from the active CLI path.

## Progress — 2026-05-14

- The active selfhost bootstrap has been refreshed after the component wrapper
  work, so `--emit component` is available on the normal selfhost command path.
- `src/compiler/main.ark` now accepts repeated `--wit <path>` flags, validates
  that each referenced WIT file can be read, and forwards the paths through
  `CliOptions` into `driver::DriverConfig`.
- `src/compiler/driver.ark` now carries `wit_paths` in `DriverConfig`, rejects
  `--emit component --target wasm32-wasi-p1` with
  `component model requires --target wasm32-wasi-p2`, and implements a minimal
  `--emit wit` path from source-level export type annotations.
- `src/compiler/main.ark` now implements `--emit all` for compile: it emits a core
  `.wasm` and sibling `.component.wasm` output. With `-o foo.wasm`, the component
  output is `foo.component.wasm`.

**Evidence:**

- `ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm
  scripts/run/arukellt-selfhost.sh compile
  tests/component-interop/jco/calculator/calculator.ark --emit all --wit
  tests/fixtures/component/import_flags_type.wit -o state/tmp_setup/all_smoke.wasm`
  writes both `state/tmp_setup/all_smoke.wasm` and
  `state/tmp_setup/all_smoke.component.wasm`.
- `wasmtime run --wasm gc --wasm component-model --invoke 'add(2, 5)'
  state/tmp_setup/all_smoke.component.wasm` prints `7`.
- `ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm
  scripts/run/arukellt-selfhost.sh compile
  tests/component-interop/jco/calculator/calculator.ark --emit wit -o
  state/tmp_setup/calculator.wit` writes a WIT world containing `add`, `mul`, and
  `negate` exports.
- `--emit wit` over the component fixture surface now preserves source-level WIT
  shapes for `bool`, `char`, `string`, `list<s32>`, `option<s32>`,
  `result<s32, string>`, `tuple<s32, s32>`, `record point`, `enum color`, and
  `variant shape` instead of reducing them to scalar `s32`.
- `--wit` import validation now rejects unsupported `flags` declarations with
  `E0090`, so `tests/fixtures/component/import_flags_type.ark --emit component
  --wit tests/fixtures/component/import_flags_type.wit` fails instead of silently
  producing a component with ignored imports.
- Missing WIT input reports
  `error: unable to read WIT file state/tmp_setup/missing.wit: file open error`.

**Still open:**

- The WIT paths are not yet consumed by resolver/typecheck/MIR to bind imported
  host functions. This issue should remain open until a WIT-import fixture proves
  that `--wit host.wit` creates callable imports during component compilation.

## Progress — 2026-05-14 (import guard)

- Added `tests/fixtures/component/import_scalar_func.{ark,wit,flags,diag}` as a
  compile-error fixture for scalar WIT function imports.
- `src/compiler/driver.ark` now rejects WIT files containing function imports
  with `E0401` instead of silently ignoring the import list and emitting a
  component with no host binding.
- This is guardrail progress, not closure evidence: callable WIT import binding
  still needs resolver symbols, type signatures, MIR import entries, core Wasm
  imports, and component canonical lowering.

**Evidence:**

- `ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm scripts/run/arukellt-selfhost.sh compile tests/fixtures/component/import_scalar_func.ark --target wasm32-wasi-p2 --emit component --wit tests/fixtures/component/import_scalar_func.wit -o state/tmp_setup/import_scalar_func.component.wasm`
  exits non-zero with `E0401: WIT function imports are not yet bound to resolver/MIR`.

## Blocked reclassification — 2026-05-15

This issue is no longer an implementation-ready open queue item. The remaining
closure requirement is not just CLI flag threading: a fixture must prove that a
WIT-imported function can be referenced from Arukellt source, typechecked, lowered
to MIR, emitted as a core Wasm import, and exposed through component canonical
lowering.

Current evidence shows the missing prerequisite:

- `tests/fixtures/component/import_scalar_func.ark` contains only `fn main() {}`.
  It can verify WIT file validation/guardrails, but it cannot prove callable
  imports because the source never names `host-add`.
- `tests/fixtures/component/import_scalar_func.wit` declares
  `import host-add: func(a: s32, b: s32) -> s32;`.
- #124 defines the missing `import "..." as alias` source syntax and generated
  namespace semantics needed to write an Arukellt call such as
  `host::add(2, 5)` against that WIT import.

Therefore #034 is blocked on #124. Keep the existing guardrail fixtures in place:
WIT function imports should continue to fail with `E0401` until source syntax and
resolver/MIR import binding are implemented together.

## Summary

Complete the end-to-end CLI workflow for Component Model usage. Add `--wit <path>`
flag for import binding, update `--emit component` to produce `.component.wasm`,
update all relevant documentation, and write the v1→v2 migration guide.

## Context

The CLI (`crates/arukellt/src/main.rs`) currently supports `--emit core-wasm` and
`--emit wit`. The `--emit component` path hits a hard error. The `--wit` flag for
specifying import WIT files does not exist.

The user-facing workflow for v2 component usage should be:

```bas

Generate WIT from Arukellt source (existing, works)
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
