---
Status: open
Created: 2026-06-12
Updated: 2026-06-12
ID: 645
Track: playground
Depends on: 632
Orchestration class: design-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 645 — T2 playground execution and DOM I/O surface

## Summary

ADR-017 and ADR-020 define T2 freestanding playground execution and minimal DOM I/O as v2 goals. T2 is compile-only scaffold today (current-state.md).

## Evidence source

docs/adr/ADR-017-playground-execution-model.md, docs/adr/ADR-020-t2-io-surface.md, docs/current-state.md §T2

## Primary paths

playground/, src/compiler/emitter.ark, tests/fixtures/t2/, docs/playground/

## Non-goals

Full WASI on T2, replacing TypeScript engine (#632 path)

## Acceptance

- [ ] Playground can execute minimal T2-compiled module in browser (smoke program)
- [ ] DOM I/O surface documented and wired per ADR-020 non-goals boundaries
- [ ] docs/playground/ and ADR-017 status rows updated with repo proof

## Required verification

```bash
python3 scripts/manager.py verify fixtures
playground npm test
```

## Close gate

Browser smoke runs T2 wasm; docs no longer claim T2 execution as future-only without tracking.
