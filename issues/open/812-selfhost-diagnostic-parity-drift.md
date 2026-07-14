---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 812
Track: selfhost
Depends on: none
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: CQ-18 audit — unresolved verify full failures need open owner
---

# 812 — Selfhost diagnostic parity drift

## Summary

`verify full` reports 3 selfhost diagnostic parity failures. The selfhost
compiler produces different diagnostic messages than the reference for 3
diagnostic cases.

## Exact failure scope

3 diagnostic cases produce different messages between the selfhost
compiler and the reference. The specific cases are captured in the verify
full receipt.

## Machine-readable baseline

`python3 scripts/manager.py selfhost diag-parity` produces the list of
mismatched diagnostics.

## Owner

compiler team (diagnostics)

## Removal condition

Each diagnostic case passes when the selfhost compiler produces a message
matching the reference for that case.

## Validation command

```bash
python3 scripts/manager.py selfhost diag-parity
```

## Current count

3 mismatched diagnostics

## New-failure ratchet

No new diagnostic parity mismatches may be added. The count must only
decrease.
