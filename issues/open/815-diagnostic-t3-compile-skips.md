---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 815
Track: selfhost
Depends on: none
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: CQ-18 audit — unresolved skips need open owner
---

# 815 — Diagnostic/T3 compile skips (23 fixtures)

## Summary

23 fixtures are skipped in the diagnostic parity and T3 compile checks.
These fixtures have missing diagnostics or incomplete compile paths in
the selfhost compiler.

## Exact failure scope

- 23 diagnostic parity skips: fixtures where the selfhost compiler lacks
  the diagnostic implementation needed to produce the expected message.
- 3 fixture parity skips: SIMD/f64 fixtures where GC push or SIMD
  extract_lane is not implemented.

## Machine-readable baseline

`python3 scripts/manager.py selfhost diag-parity` and the fixture parity
runner produce the list of skipped fixture IDs.

## Owner

compiler team (diagnostics + wasm backend)

## Removal condition

Each skip is removed when the selfhost compiler implements the missing
diagnostic or compile path for that fixture. The fixture is removed from
the skip list and runs as a normal check.

## Validation command

```bash
python3 scripts/manager.py selfhost diag-parity
python3 scripts/manager.py selfhost fixture-parity
```

## Current count

23 diagnostic skips + 3 fixture parity skips = 26 total skips

## New-failure ratchet

No new skips may be added. The count must only decrease. Any new skip is
a regression and blocks merge.
