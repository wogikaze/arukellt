---
Status: done
Created: 2026-06-12
Updated: 2026-06-14
ID: 638
Track: runtime
Depends on: 183
Orchestration class: design-ready
Blocks v1 exit: none
Source: docs/process/docs-gap-inventory-2026-06-12.md
---

# 638 — Runtime-level Wasm debugging (source maps + wasmtime hooks)

## Summary

Current DAP support uses source-level simulated breakpoints with static variable text. docs/debug-support.md Future section requires Wasm-level breakpoint injection, source map emission, and live variable inspection.

## Evidence source

docs/debug-support.md §Future (L131-138), extensions/arukellt-all-in-one/

## Primary paths

src/compiler/emitter.ark, src/compiler/driver/, extensions/arukellt-all-in-one/, docs/debug-support.md

## Non-goals

Multi-thread debugging, conditional breakpoints, watch expressions

## Acceptance

- [x] Debug pipeline emits source map mapping Wasm offsets to source lines for at least T1/T3 smoke programs
- [x] Wasmtime debug hook integration enables true runtime breakpoints
- [x] DAP variables response includes live runtime values (not static placeholder text) for smoke program
- [x] docs/debug-support.md Limitations section updated to reflect achieved vs remaining gaps

## Required verification

```bash
python3 scripts/manager.py verify quick
# extension E2E debug smoke when available
```

## Close gate

Repo proof: source map artifact + DAP session log showing live variable values on stepping.

## Close note

- `tools/host-linker` post-links `metadata.debug.source_map` and `arukellt_debug::breakpoint` hooks for smoke/core Wasm modules.
- `tools/host-linker` provides wasmtime debug runner + `arukellt-debug-adapter` DAP (stdio) with live locals.
- Gate: `scripts/check/check-wasm-debug-smoke.py` (smoke fixture `tests/fixtures/selfhost/debug_smoke.ark`).
