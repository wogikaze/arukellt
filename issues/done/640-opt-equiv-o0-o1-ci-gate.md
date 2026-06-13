---
Status: done
Created: 2026-06-12
Updated: 2026-06-12
ID: 640
Track: wasm-quality
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 640 — opt-equiv CI gate (O0 == O1 semantics)

## Summary

release-checklist.md defers opt-equiv (O0 == O1) gate. scripts/run/test-opt-equivalence.sh exists but is not wired into manager.py verify as a hard gate.

## Evidence source

docs/release-checklist.md L14, scripts/run/test-opt-equivalence.sh, scripts/manager.py

## Primary paths

scripts/manager.py, scripts/run/test-opt-equivalence.sh, docs/release-checklist.md

## Non-goals

O2/O3 equivalence, performance regression gates

## Acceptance

- [x] python scripts/manager.py verify includes opt-equivalence check (or dedicated verify subcommand invoked by verify quick)
- [x] Gate fails when O1 changes program output vs O0 on covered fixtures
- [x] docs/release-checklist.md DEFERRED comment replaced with active CI checkbox

## Required verification

```bash
bash scripts/run/test-opt-equivalence.sh --quick
python3 scripts/manager.py verify quick
```

## Close gate

verify quick green with new gate; release-checklist item unchecked only until release tag process.
