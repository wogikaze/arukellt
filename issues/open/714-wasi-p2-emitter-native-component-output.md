---
Status: open
Created: 2026-07-02
Updated: 2026-07-02
ID: 714
Track: component-model
Parent: 668
Depends on: "074 (done), 510 (done)"
Orchestration class: architecture-implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: WASI 0.2 stdout architecture review 2026-07-02 — remove wrapper-only P2 component path
---

# 714 — Emitter-native WASI P2 component output without wrapper

## Summary

Arukellt's `wasm32-wasi-p2` command output must be a valid Component Model
artifact directly from the compiler/emitter. The compiler should not emit a core
wasm module with pseudo Preview 2 imports such as
`wasi:cli/stdout@0.2.0::write` and then rely on `p2_component_wrap.py` to repair
that shape after compilation.

The target behavior is:

```text
component
  imports
    wasi:cli/stdout@0.2.0
    wasi:io/streams@0.2.0
    ...
  core module
    compiled Arukellt core wasm
  canonical ABI glue
    list/resource/result lowering between the component world and core wasm
  exports
    wasi:cli/run@0.2.0
```

## Problem

The current P2 command path is architecture-inverted:

- `src/compiler/wasm/sections_imports.ark` emits direct core imports like
  `wasi:cli/stdout@0.2.0::write`.
- In WASI 0.2, `wasi:cli/stdout` provides `get-stdout`; byte writes are methods
  on `wasi:io/streams` `output-stream` resources.
- `scripts/selfhost/p2_component_wrap.py` patches/adapts the generated core wasm
  into a component by wiring host `get-stdout` to stream write/flush behavior.

That helper was useful to prove the first P2 native command path, but it should
not remain the product contract. A compiler targeting WASI 0.2 should emit the
component, WIT-shaped imports, canonical ABI glue, and `wasi:cli/run` export
directly.

## Goals

1. Emit a valid `wasi:cli/command` Component Model binary for
   `wasm32-wasi-p2` without invoking `p2_component_wrap.py`.
2. Replace pseudo core imports such as `wasi:cli/stdout@0.2.0::write` with
   component-correct imports:
   - `wasi:cli/stdout@0.2.0.get-stdout`
   - `wasi:io/streams@0.2.0` `output-stream` methods needed by stdio
3. Generate or reuse canonical ABI glue in the emitter for:
   - `list<u8>` / guest memory lowering
   - resource handles for `own<output-stream>`
   - `result` / error return shapes used by stream writes
   - `wasi:cli/run@0.2.0` export shape
4. Remove `p2_component_wrap.py` from the normal compile/run/verify path.
5. Keep `wasm-tools validate` and `wasmtime run` evidence for the wrapper-free
   component path.

## Non-goals

- Full WASI filesystem, sockets, HTTP, or async coverage beyond the command
  stdout/stderr/env/args/exit surface needed by existing P2 native gates.
- General library component interop fixes tracked by #667 and related
  component export issues.
- Full `std::wit` WIT parser consolidation tracked by #706.
- T4 native or LLVM component output.

## Acceptance

- [ ] `wasm32-wasi-p2` command compilation emits a Component Model binary
      directly; no post-compile `p2_component_wrap.py` invocation is required.
- [ ] Generated P2 command components import `wasi:cli/stdout@0.2.0` and
      `wasi:io/streams@0.2.0` using WASI 0.2 interface/resource semantics, not a
      direct `wasi:cli/stdout@0.2.0::write` core function import.
- [ ] `tests/fixtures/wasi_p2_native/hello.ark` validates with `wasm-tools
      validate` and prints expected stdout under `wasmtime run` using the
      wrapper-free artifact.
- [ ] At least one stderr or exit-code fixture proves the same emitter-native
      component path beyond stdout-only hello.
- [ ] `scripts/run/arukellt-selfhost.sh run` handles wrapper-free P2 command
      artifacts without shelling out to `p2_component_wrap.py`.
- [ ] `p2_component_wrap.py` is deleted, or moved to a clearly marked legacy
      fixture-only location with no production or verify dependency.
- [ ] `docs/current-state.md` and target/component docs describe emitter-native
      P2 component output; no docs claim wrapper repair is the normal path.
- [ ] #668 is updated or closed against this coherent architecture decision.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Close gate

Add or update a gate under `scripts/check/` so it:

1. Builds at least the P2 native hello fixture.
2. Fails if the produced artifact contains a direct
   `wasi:cli/stdout@0.2.0::write` core import.
3. Fails if the proof path invokes `scripts/selfhost/p2_component_wrap.py`.
4. Validates the artifact as a component with `wasm-tools validate`.
5. Runs it with Wasmtime and asserts expected stdout.

## Dependency Notes

- Depends on #074 for the original P2 native component proof and stdout bridge
  lessons.
- Depends on #510 for the existing target-mode import table switch that exposed
  the P2-specific import surface.
- Blocks the coherent-architecture portion of #668.
- Related but not blocking: #667 (library component routing), #678
  (verification gate coverage), #706 (`std::wit` parser compliance).

## References

- `issues/done/074-wasi-p2-native-component.md`
- `issues/done/510-t3-p2-import-table-switch.md`
- `issues/open/668-p2-native-component-polish.md`
- `src/compiler/wasm/sections_imports.ark`
- `src/compiler/wasm/component_p2_emit.ark`
- `src/compiler/component/component_base.ark`
- `scripts/selfhost/p2_component_wrap.py`
- `scripts/run/arukellt-selfhost.sh`
