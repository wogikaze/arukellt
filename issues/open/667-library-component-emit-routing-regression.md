---
Status: open
Created: 2026-06-16
Updated: 2026-06-16
ID: 667
Track: component-model
Depends on: 666
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: #666 close-out review — bootstrap driver patch bypasses specialized component emitters for library modules
---

# 667 — Library component routing: scalar emitter bypasses specialized / WIT-complete path

## Summary

Issue #666 added a **bootstrap-safe scalar library component emitter**
(`src/compiler/wasm/library_component_emit.ark`) and a bootstrap overlay driver patch
(`scripts/selfhost/checks.py::_patch_bootstrap_driver_component_delegate`) that routes
**all** `mir_has_library_exports` modules to `wasm::emit_library_component` instead of
`component::emit_component`.

That unblocked `examples/ark/export-library/calculator.ark` (`i32` `add`/`mul`) on s2
selfhost, but it **regressed** every library-shaped fixture that relies on
`component::emit_specialized` (string, record, list, option, result, tuple, enum, variant).

WIT text emit was fixed in #666; **component emit for non-scalar library exports is not
complete**. The scalar shortcut must not become the permanent library path.

## Why the scalar emitter exists (#666 context)

| Constraint | What happened |
|------------|---------------|
| Bootstrap overlay memory budget | Full `component::emit_component` generic export path hit `unreachable` / symbol collisions in overlay s2 |
| P2 command stub | `BOOTSTRAP_COMPONENT_STUB` still wraps command apps; library exports needed a separate minimal path |
| Short-term goal | Ship invokable `i32` library components for `calculator.ark` without `wasm-tools embed` |

`library_component_emit.ark` was intentionally **minimal**: alias discriminator fix + scalar
canonical ABI only (`s32` params/results). It does not implement string/record/list adapters.

The production source already has `component::emit_library_component` in
`src/compiler/component/emit.ark`, which **tries `emit_specialized` first** and only then
falls back to generic export sections. The bootstrap driver patch **skips that entirely**.

## Problem

### A. Specialized emitters bypassed on s2 recompile

Modules with `pub fn` exports **and** `fn main` (or any exportable `pub fn` surface) are
classified as library exports. On s2 selfhost, the patched driver calls
`wasm::emit_library_component` directly.

**Observed regression** (s2 recompile):

| Fixture | Expected WIT | Actual WIT (s2) |
|---------|--------------|-----------------|
| `string-greet` | `greet: func(name: string) -> string` | `greet: func(arg0: s32) -> s32` |
| `record-point` | record params/results | all `s32` |

`wasmtime --invoke` then fails at argument parsing — not because canonical ABI string
adapters are missing (#121/#660 implemented fixture shapes), but because the **wrong emitter**
ran.

### B. “Fixtures pass” is misleading

- `python3 scripts/manager.py verify quick` passes (166/166).
- Most `tests/component-interop/jco/*/run.sh` still default to
  `ARUKELLT_BIN=target/debug/arukellt` or use committed `.component.wasm` without s2
  recompile.
- Only `jco/calculator/run.sh` was updated to s2 selfhost in #666 close-out.
- `docs/current-state.md` lists 103/103 interop fixtures as passing — that reflects the
  **pre-recompile / debug-compiler** gate, not s2 full recompile of every library-shaped
  fixture.

### C. Bootstrap fixpoint drift

After #666, `.build/selfhost/arukellt-s2.wasm` (~1.3 MiB) no longer matches
`bootstrap/arukellt-selfhost.wasm` (~900 KiB). `selfhost fixpoint` is skipped / not reached.
Pinned refresh policy (`bootstrap/PROVENANCE.md`) was not applied.

### D. WIT vs component asymmetry

Issue 666 fixed `--emit wit` for scalar libraries on s2. Users may assume `--emit component`
has the same coverage; it does not for string/record/list library modules on s2.

## Goals

1. **Route library component emit through specialized dispatch** — library modules with
   non-scalar export shapes must reach `component::emit_specialized` (or
   `component::emit_library_component`), not `wasm::emit_library_component` alone.
2. **Retain scalar fast path only when appropriate** — `i32`/`i64`/`f64`/`bool`-only
   library worlds may use `library_component_emit.ark` after shape detection says no
   specialized emitter applies.
3. **Honest gates** — s2 recompile smoke for at least one string and one record library
   fixture; document which jco `run.sh` paths require s2 vs debug compiler.
4. **Fixpoint policy** — either restore `sha256(pinned) == sha256(s2)` after intentional
   refresh, or document temporary drift with a tracked refresh sub-task.

## Non-goals

- Full Tier 2/3 general canonical ABI beyond #121/#648/#660 boundaries (E0401 matrix
  unchanged).
- Removing `BOOTSTRAP_COMPONENT_STUB` entirely (fixpoint memory budget).
- jco / Node interop (#036, blocked #037).

## Acceptance

### Routing

- [ ] s2 selfhost: `string-greet.ark --emit component` produces WIT with `string` param/result (not `s32`).
- [ ] s2 selfhost: `record-point/point.ark --emit component` produces WIT with record types (not flat `s32`).
- [ ] s2 selfhost: `calculator.ark --emit component` still invokes `add(3,4) -> 7` (no regression).
- [ ] Bootstrap overlay driver patch calls `component::emit_library_component` (specialized-first) or equivalent — not bare `wasm::emit_library_component` for all library modules.

### Tests / gates

- [ ] `tests/component-interop/jco/string-greet/run.sh` passes with s2 selfhost recompile (or documented SKIP only when s2 absent).
- [ ] `tests/component-interop/jco/record-point/run.sh` passes with s2 selfhost recompile.
- [ ] Optional gate: `scripts/check/gate-667-library-specialized-routing.py` — compile string + record fixtures on s2, assert WIT shapes.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

### Docs

- [ ] `docs/current-state.md` — clarify library `--emit component` scalar vs specialized routing; note #667 fixes s2 recompile regression for string/record fixtures.
- [ ] `issues/done/666-*.md` addendum or cross-link if acceptance wording implied full library component coverage.

### Bootstrap (choose one)

- [ ] `sha256(pinned) == sha256(s2)` after explicit pinned refresh + `bootstrap/PROVENANCE.md` update, **or**
- [ ] Known Limitations documents fixpoint drift until refresh lands (with issue sub-checkbox left open).

## Implementation notes

### Suggested fix

```ark
// driver emit (bootstrap overlay and production should align)
let comp_bytes = if component::mir_has_library_exports(mir_module) {
    component::emit_library_component(core_wasm, mir_module, target, wasi_version, world)
} else {
    component::emit_component(...)
}
```

Inside `component::emit_library_component` (already in `emit.ark`):

1. `emit_specialized_component` — string, record, list, …
2. `emit_component_generic_exports` — scalar generic plan
3. Optional: delegate to `wasm::emit_library_component` only when export plan is
   all Tier-1 scalars and generic path would duplicate the minimal emitter

### Key files

| Path | Role |
|------|------|
| `scripts/selfhost/checks.py` | `_patch_bootstrap_driver_component_delegate`, `BOOTSTRAP_COMPONENT_STUB` |
| `src/compiler/driver/emit.ark` | production `emit_component_core_wasi_version` (`p1-component` for libraries) |
| `src/compiler/component/emit.ark` | `emit_library_component` (specialized-first) |
| `src/compiler/wasm/library_component_emit.ark` | scalar-only minimal emitter (#666) |
| `src/compiler/component/emit_specialized.ark` | fixture-shaped adapters (#121 matrix) |

## Required verification

```bash
ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
  scripts/run/arukellt-selfhost.sh compile \
  tests/component-interop/jco/string-greet/string_greet.ark \
  --target wasm32-wasi-p2 --emit component -o /tmp/sg.component.wasm
wasm-tools component wit /tmp/sg.component.wasm | grep -q 'string'

ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
  scripts/run/arukellt-selfhost.sh compile \
  examples/ark/export-library/calculator.ark \
  --target wasm32-wasi-p2 --emit component -o /tmp/calc.component.wasm
wasmtime run --wasm gc --wasm component-model --invoke 'add(3, 4)' /tmp/calc.component.wasm

python3 scripts/manager.py verify quick
```

## References

- `issues/done/666-component-wit-emit-library-exports.md` — scalar library path (closed; routing regression discovered post-close)
- `issues/done/121-wasi-p2-canonical-abi-hardening.md` — specialized fixture adapters
- `issues/done/660-component-export-tier2-general-adapters.md` — Tier 2 string general adapter boundary
- `docs/process/false-done-prevention.md` — FD-07 bootstrap stub vs production behaviour

## Close gate

s2 recompile of `string-greet` and `record-point` produces correct WIT and wasmtime invoke
passes; `calculator` scalar library path unchanged; docs and optional gate-667 green;
fixpoint refresh completed or explicitly documented.
