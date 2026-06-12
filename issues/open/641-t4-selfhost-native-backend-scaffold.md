---
Status: open
Created: 2026-06-12
Updated: 2026-06-12
ID: 641
Track: runtime
Depends on: none
Orchestration class: design-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 641 — T4 selfhost-native backend scaffold

## Summary

T4 (native) is not-implemented per docs/current-state.md and docs/target-contract.md. ark-llvm scaffold was removed (#586). Future backend will be selfhost-native, distinct from #529 Phase 7 crate deletion work.

## Evidence source

docs/current-state.md L39, docs/target-contract.md T4, docs/migration/t1-to-t3.md

## Primary paths

src/compiler/driver.ark, src/compiler/emitter.ark, src/compiler/target.ark, docs/target-contract.md

## Non-goals

Production native linker, full std::host on native, #529 Rust crate deletion

## Acceptance

- [ ] --target native compiles a minimal smoke program to object/asm artifact (compile-only acceptable)
- [ ] Driver registers native target with status scaffold (not not-implemented error at CLI parse)
- [ ] docs/current-state.md and docs/target-contract.md T4 status updated to scaffold with honest run_supported=false
- [ ] At least one tests/fixtures/ entry documents native scaffold compile proof

## Required verification

```bash
python3 scripts/manager.py verify quick
```

## Close gate

Native scaffold fixture compiles; docs target table matches driver registration.
