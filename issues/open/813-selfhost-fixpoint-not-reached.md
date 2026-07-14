---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 813
Track: selfhost
Depends on: "459 (done, framework)"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: CQ-18 audit — unresolved verify full failures need open owner
---

# 813 — Selfhost fixpoint not reached

## Summary

`verify full` reports 1 selfhost fixpoint failure. The selfhost compiler
does not reach a fixpoint (s2 != s3 binary identity).

## Exact failure scope

The selfhost fixpoint check fails: compiling the source with s2 produces
s3, and sha256(s2) != sha256(s3). The specific divergence is captured in
the verify full receipt.

## Machine-readable baseline

`python3 scripts/manager.py selfhost fixpoint` produces the s2/s3 hash
comparison.

## Owner

compiler team (selfhost)

## Removal condition

Fixpoint is reached when sha256(s2) == sha256(s3) for the current source
tree.

## Validation command

```bash
python3 scripts/manager.py selfhost fixpoint
```

## Current count

1 fixpoint failure (s2 != s3)

## New-failure ratchet

The fixpoint must be reached. No regression from fixpoint-achieved to
fixpoint-lost is acceptable.
