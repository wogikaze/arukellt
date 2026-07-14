---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 811
Track: selfhost
Depends on: none
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: CQ-18 audit — unresolved verify full failures need open owner
---

# 811 — Selfhost CLI parity drift

## Summary

`verify full` reports 3 selfhost CLI parity failures. The selfhost
compiler's CLI output differs from the reference for 3 commands.

## Exact failure scope

3 CLI commands produce different output between the selfhost compiler and
the reference. The specific commands are captured in the verify full
receipt.

## Machine-readable baseline

`python3 scripts/manager.py selfhost cli-parity` produces the list of
mismatched commands.

## Owner

compiler team (CLI)

## Removal condition

Each CLI command passes when the selfhost compiler produces output
matching the reference for that command.

## Validation command

```bash
python3 scripts/manager.py selfhost cli-parity
```

## Current count

3 mismatched commands

## New-failure ratchet

No new CLI parity mismatches may be added. The count must only decrease.
