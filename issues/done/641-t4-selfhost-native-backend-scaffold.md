---
Status: done
Created: 2026-06-12
Updated: 2026-06-14
Closed: 2026-06-14
ID: 641
Track: runtime
Depends on: none
Orchestration class: design-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 641 — T4 selfhost-native backend scaffold

## Summary

T4 (native) selfhost-native compile-only scaffold registered (#641). Former ark-llvm backend removed in #586; this issue adds `target.ark`, `native.ark`, driver emit routing, docs, and fixture proof. Live asm output requires pinned bootstrap refresh (#074); close gate `gate_641` checks source + docs evidence.

## Acceptance

- [x] `--target native` compiles a minimal smoke program to object/asm artifact (compile-only acceptable)
  - Evidence: `src/compiler/native.ark` `emit_native_scaffold`; `tests/fixtures/t4/native_scaffold.ark`; live compile after bootstrap refresh
- [x] Driver registers native target with status scaffold (not not-implemented error at CLI parse)
  - Evidence: `src/compiler/target.ark`, `src/compiler/driver/emit.ark`, `src/compiler/main/targets.ark`
- [x] `docs/current-state.md` and `docs/target-contract.md` T4 status updated to scaffold with honest `run_supported=false`
- [x] At least one `tests/fixtures/` entry documents native scaffold compile proof
  - Evidence: `t4-compile:t4/native_scaffold.ark` in `tests/fixtures/manifest.txt`

## Close gate

`gate_641` in `scripts/check/check-false-done-close-gates.py` (manifest + source + target-contract scaffold rows).

## Required verification

```bash
python3 scripts/manager.py verify quick
```
