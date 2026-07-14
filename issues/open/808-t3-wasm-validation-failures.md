---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 808
Track: compiler
Depends on: "686"
Orchestration class: ready
Orchestration upstream: 686
Blocks v{N}: none
Priority: 3
Source: CQ-18 audit — unresolved verify full failures need open owner
---

# 808 — T3/Wasm validation failures

## Summary

`verify quick` reports 1 aggregate failure: "T3 fixture WASM validation
gate (#686)". This expands to 192 individual fixture failures where the
selfhost compiler produces Wasm that fails `wasm-tools validate`.

## Exact failure scope

192 fixtures fail T3 WASM validation. The selfhost compiler emits invalid
Wasm for these fixtures. The `check-t3-wasm-validate.py` script aggregates
these into a single failure for `verify quick`.

## Machine-readable baseline

`python3 scripts/check/check-t3-wasm-validate.py --json` produces the list
of failing fixture IDs.

## Owner

compiler team (wasm backend)

## Removal condition

Each fixture passes when the selfhost compiler emits valid Wasm that
`wasm-tools validate` accepts. The fixture is removed from the failing set
when T3 validation passes.

## Validation command

```bash
python3 scripts/check/check-t3-wasm-validate.py
```

## Current count

192 failing fixtures (1 aggregate in verify quick)

## New-failure ratchet

No new T3 validation failures may be added. The count must only decrease.
Any increase is a regression and blocks merge.
