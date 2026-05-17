---
Status: open
Created: 2026-05-17
Updated: 2026-05-17
ID: 632
Track: playground
Depends on: 501
Orchestration class: implementation-ready
Blocks v1 exit: no
Blocks v2: yes
Priority: 70
Implementation target: "Use the selfhost compiler Wasm as the browser compiler; do not implement language execution semantics in TypeScript."
Source: "User correction: playground Build/Run must use the compiler Wasm in the browser and a Node-like compile/run host model, not a feature-by-feature TypeScript interpreter."
---

# Playground compiler-Wasm build/run loop

## Summary

Implement playground v2 Build/Run using the real selfhost compiler Wasm in a
browser worker and a T2 browser runner. The TypeScript layer must orchestrate
compiler process execution, virtual files, stdio buffers, and Wasm instantiation;
it must not reimplement Arukellt language semantics.

Design authority: [ADR-032](../../docs/adr/ADR-032-playground-compiler-wasm-runner.md).

## Problem

The current playground engine is a TypeScript parse/format/tokenize/typecheck
surface. It is useful for v1 feedback, but it is not a correct execution model.
Adding support for language features one by one in TypeScript would create a
second interpreter and drift from the selfhost compiler.

The desired model is:

1. browser loads a compiler Wasm asset
2. worker runs the compiler as a command with virtual FS/stdio
3. compiler emits `wasm32-freestanding` output
4. browser runner instantiates the emitted Wasm with `arukellt_io` stdio imports
5. UI displays compiler diagnostics, build artifact metadata, stdout, stderr,
   traps, and exit code

## Scope

### Slice A — compiler asset packaging

- Publish `bootstrap/arukellt-selfhost.wasm` or the selected stage-2 selfhost
  compiler artifact into `docs/playground/assets/` during `npm run build:app`.
- Record the copied asset size and sha256 in a generated or checked metadata
  file.
- Keep the retired `crates/ark-playground-wasm` path deleted.

### Slice B — compiler worker process host

- Add a browser worker API for:
  - `compile(source, options)`
  - target defaults to `wasm32-freestanding`
  - virtual input path `/work/main.ark`
  - virtual output path `/work/out.wasm`
- Provide argv/env, stdout/stderr capture, and an in-memory filesystem.
- Enforce timeout and byte-size limits.
- Add a Node harness that exercises the same compile request contract.

### Slice C — T2 stdio lowering

- Extend the selfhost T2 emitter beyond the current scaffold so `std::host::stdio`
  output lowers to `arukellt_io` imports.
- Export linear memory as `"memory"`.
- Keep T2 WASI-free.
- Add fixture proof that a stdio program compiles to a module with the expected
  `arukellt_io` imports and validates as core Wasm.

### Slice D — browser T2 runner

- Add a runner that instantiates emitted T2 bytes with:
  - `arukellt_io.write`
  - `arukellt_io.write_err`
  - `arukellt_io.flush`
  - `arukellt_io.flush_err`
  - `arukellt_io.read`
- Capture stdout, stderr, exit code or trap text.
- Add Node tests for a known-good fixture once Slice C can emit stdio imports.

### Slice E — UI integration

- Wire `docs/playground/index.html` Build/Run buttons to the compiler worker
  and T2 runner.
- Show compiler stderr separately from program stderr.
- Show disabled/unavailable states when the compiler asset is absent, the worker
  cannot load, or required Wasm features are unavailable.

## Acceptance

- [ ] `npm --prefix playground run build:app` produces a compiler Wasm asset under `docs/playground/assets/`.
- [ ] A Node test compiles a source string by invoking the compiler Wasm through the worker/process-host contract.
- [ ] `compile()` returns output Wasm bytes, compiler stdout/stderr, exit code, and diagnostics metadata.
- [ ] T2 stdio output lowers to `arukellt_io` imports; fixture proof validates the emitted module.
- [ ] A runner test executes a compiled T2 stdio fixture and captures stdout/stderr without a TypeScript interpreter.
- [ ] `docs/playground/index.html` exposes Build/Run through the real compiler/runner path only.
- [ ] `python3 scripts/check/check-docs-consistency.py` passes.
- [ ] `python scripts/manager.py verify quick` passes.

## STOP_IF

- The implementation starts interpreting Arukellt source in TypeScript.
- The solution depends on the retired `crates/ark-playground-wasm` crate.
- The browser worker requires network access or persistent filesystem access for
  user programs.
- T2 cannot emit runnable stdio Wasm yet; in that case only complete Slice A/B
  and leave Run disabled with an honest unavailable state.
