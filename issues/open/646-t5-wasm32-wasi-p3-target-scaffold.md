---
Status: open
Created: 2026-06-12
Updated: 2026-06-12
ID: 646
Track: wasm-feature
Depends on: 474
Orchestration class: design-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 646 — T5 wasm32-wasi-p3 target scaffold

## Summary

T5 target id exists but has no backend, runtime, or scaffold per docs/current-state.md. Distinct from #474 async component v5 work.

## Evidence source

docs/current-state.md L40, docs/target-contract.md T5

## Primary paths

src/compiler/driver.ark, src/compiler/target.ark, docs/target-contract.md

## Non-goals

Full WASI Preview 3 runtime, async component export (#474)

## Acceptance

- [ ] Driver registers wasm32-wasi-p3 with honest not-started → scaffold transition documented
- [ ] CLI --target wasm32-wasi-p3 produces clear error or compile-only scaffold (documented behavior)
- [ ] docs/current-state.md and target-contract.md T5 rows synced

## Required verification

```bash
python3 scripts/manager.py verify quick
```

## Close gate

Target registration + docs alignment; no silent mis-targeting.
