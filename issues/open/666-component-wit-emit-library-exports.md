---
Status: open
Created: 2026-06-16
Updated: 2026-06-16
ID: 666
Track: component-model
Depends on: 074, 618
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: examples/ark/export-library workaround; bootstrap component stub vs production emitter gap
---

# 666 — `--emit wit` / `--emit component` library export completeness + examples

## Summary

`--emit wit` and `--emit component` are documented as the primary path for exporting
`pub fn` surfaces to the Component Model, but **library-style modules** (e.g.
`examples/ark/export-library/calculator.ark`) do not round-trip today without a manual
`wasm-tools component embed` + hand-written `.wit` workaround.

Close the gap so a single compiler invocation produces:

1. **WIT text** (`--emit wit`) describing all exportable `pub fn` declarations.
2. **Component binary** (`--emit component` / `--emit all`) with inline WIT metadata and
   invokable canonical-ABI exports — verifiable via `wasm-tools component wit` and
   `wasmtime --invoke`.

Then **simplify `examples/`** to use the native CLI path instead of the external embed
pipeline.

## Problem

### A. Bootstrap overlay stub (pinned selfhost)

Fixpoint bootstrap replaces the full component module with `BOOTSTRAP_COMPONENT_STUB`
(`scripts/selfhost/checks.py`):

- `wasi-version p2` → `emit_p2_command_component` only (command app wrapper).
- otherwise → returns **raw core wasm** (not a component).
- `emit_wit_text_from_decls` → empty string.

Pinned `bootstrap/arukellt-selfhost.wasm` therefore produces `world root {}` and
0-byte WIT for library fixtures. This is intentional for fixpoint memory but **must not
be mistaken for production behaviour** (see false-done audit FD-07, #074).

### B. Production emitter routing (`src/compiler/component/emit.ark`)

For `wasm32-wasi-p2` + default `wasi-version p2`, non-specialized modules fall through to
`emit_p2_command_component` **before** the generic export plan:

```ark
if world_spec::world_spec_uses_p2_command_component(...) {
    return wasm::emit_p2_command_component(core_wasm)
}
// generic alias / canon / export sections never reached for library exports
```

Library exports (`pub fn add(a: i32, b: i32) -> i32`) that do not match a fixture-shaped
specialized emitter therefore lack component exports and invokable ABI adapters.

### C. Committed interop artifacts drift

`tests/component-interop/jco/*/*.component.wasm` includes working binaries (e.g.
`i32_renamed.component.wasm` with `export offset-count`) and broken ones
(`calculator.component.wasm` with empty `world root {}`). Recompiling with current s2
selfhost does not reproduce the working artifacts.

### D. Examples workaround

`examples/ark/export-library/` currently documents a **four-step external pipeline**
(core wasm → hand-written `calculator.wit` → `wasm-tools component embed` →
`component new` + WASI adapter). This contradicts user-facing docs
(`docs/migration/v1-to-v2.md`, `docs/quickstart.md`) that show:

```bash
arukellt compile --target wasm32-wasi-p2 --emit component mylib.ark
arukellt compile --target wasm32-wasi-p2 --emit wit mylib.ark
```

## Goals

1. **Library vs command disambiguation** — route `main` / `wasi:cli/command` programs to
   P2 command wrapping; route export-only libraries through generic / scalar canonical ABI
   export (or a dedicated `library` world).
2. **`--emit wit` completeness** — non-empty WIT from `pub fn` export surface on s2+
   selfhost; golden diff tests for scalar fixtures.
3. **`--emit component` completeness** — produced `.component.wasm` embeds WIT with
   exports; `wasmtime --invoke 'add(3, 4)'` succeeds for
   `examples/ark/export-library/calculator.ark` without `wasm-tools embed`.
4. **Bootstrap honesty** — either document pinned-bootstrap limitation in CLI diagnostic
   (`warning: component emit uses bootstrap stub; build s2 for full output`) or gate
   `--emit component` / `--emit wit` on non-stub compiler with actionable message.
5. **Examples refresh** — update `examples/{ark,rust,js}/` README + `run.sh` to prefer
   native `--emit component`; keep embed path only as optional “manual WIT” appendix.

## Non-goals

- Full Tier 2/3 canonical ABI (string/list/record general adapters) — tracked under #121,
  #648, #660.
- jco in-process Node smoke (#036 / #037).
- Removing bootstrap stub entirely (fixpoint memory budget) — only clarify boundaries.

## Acceptance

### Compiler / WIT

- [x] `arukellt compile examples/ark/export-library/calculator.ark --target wasm32-wasi-p2 --emit wit -o /tmp/calc.wit` writes non-empty WIT listing `add` and `mul` (s2 selfhost).
- [x] Golden WIT diff: `examples/ark/export-library/calculator.expected.wit` checked in CI or via `tests/component-interop/roundtrip/` extension.
- [x] `wasm-tools component wit` on `--emit component` output lists the same exports (no empty `world root {}` for library fixtures).

### Component invoke

- [x] `wasmtime run --wasm gc --wasm component-model --invoke 'add(3, 4)' <calc.component.wasm>` → `7` from **fresh** `--emit component` build (s2 selfhost, no external embed).
- [ ] `tests/component-interop/jco/calculator/run.sh` recompiles and passes invoke tests (or fixture wasm regenerated from fixed emitter).
- [ ] Library + `main` in same module: documented behaviour (export library world vs command world); compile error or dual-world policy explicit in docs.

### Bootstrap boundary

- [x] Pinned bootstrap either emits diagnostic when `--emit wit`/`component` would use stub, or close-gate documents permanent compile-only stub with redirect to s2 build instructions.
- [x] `docs/current-state.md` Known Limitations updated: library `--emit wit` and scalar library `--emit component` require s2 (#666).

### Examples

- [x] `examples/ark/export-library/run.sh` uses `--emit component` (or `--emit all`) as primary path; manual `wasm-tools embed` moved to “Appendix: external WIT”.
- [x] `examples/ark/export-library/README.md` quickstart matches post-fix CLI (remove embed as default).
- [ ] `examples/README.md` diagram updated if pipeline simplifies.
- [ ] `docs/quickstart.md` cross-link still valid; no contradiction with examples.

### Regression

- [ ] Existing specialized component interop fixtures (`tests/component-interop/jco/*`) still pass after recompile or wasm refresh.
- [ ] `python3 scripts/manager.py verify quick` exits 0.
- [ ] `python3 scripts/check/check-docs-consistency.py` exits 0.

## Implementation notes

### Suggested routing fix (`emit.ark`)

```
if has_exportable_pub_fns && !is_command_world:
    generic_export_path(plan)
else if world_spec_uses_p2_command_component:
    emit_p2_command_component
else:
    generic_export_path(plan)
```

`has_exportable_pub_fns` can reuse `export_plan::collect_component_exports`.

### Scalar library path

Short term: extend generic export sections for `i32`/`i64`/`f64`/`bool` unary/binary
exports before requiring every shape to match `emit_specialized` fixture detectors.

### Tests to add / extend

| Path | Purpose |
|------|---------|
| `tests/component-interop/jco/calculator/` | Recompile + invoke gate (already exists; make green) |
| `examples/ark/export-library/calculator.expected.wit` | WIT golden for `--emit wit` |
| `scripts/check/gate-666-component-library-emit.py` | Optional close gate: calculator ark → wit + component + wasmtime |

## Required verification

```bash
ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
  scripts/run/arukellt-selfhost.sh compile \
  examples/ark/export-library/calculator.ark \
  --target wasm32-wasi-p2 --emit wit -o /tmp/calc.wit

ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
  scripts/run/arukellt-selfhost.sh compile \
  examples/ark/export-library/calculator.ark \
  --target wasm32-wasi-p2 --emit component \
  -o /tmp/calc.component.wasm

wasm-tools component wit /tmp/calc.component.wasm
wasmtime run --wasm gc --wasm component-model --invoke 'add(3, 4)' /tmp/calc.component.wasm

bash examples/ark/export-library/run.sh
python3 scripts/manager.py verify quick
```

## References

- `src/compiler/component/emit.ark` — export routing
- `scripts/selfhost/checks.py` — `BOOTSTRAP_COMPONENT_STUB`
- `issues/done/074-wasi-p2-native-component.md` — P2 command vs library boundary
- `issues/done/618-wit-bindings-round-trip.md` — WIT round-trip contract
- `issues/done/030-emit-component.md` — original `--emit component` acceptance
- `examples/ark/export-library/` — current workaround to remove
- `docs/process/false-done-prevention.md` — FD-07 bootstrap stub policy

## Close gate

`examples/ark/export-library/run.sh` passes using native `--emit component` only; WIT
golden matches; `docs/current-state.md` and quickstart agree; calculator jco interop
fixture recompiles green on s2 selfhost.
