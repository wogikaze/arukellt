---
Status: done
Created: 2026-07-14
Updated: 2026-07-21
ID: 812
Track: selfhost
Depends on: none
Orchestration class: done
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: CQ-18 audit — unresolved verify full failures need open owner
---

# 812 — Selfhost diagnostic parity drift

## Summary

`verify full` reported 3 selfhost diagnostic parity failures for target-gating
fixtures. Closed after aligning `.diag` goldens with the current E0500 wording
(host profile `wasi-p2` instead of legacy `(T3)` phrasing).

## Validation command

```bash
python3 scripts/manager.py selfhost diag-parity
```

## Close evidence (2026-07-21)

```text
✓ selfhost diagnostic parity
```

Updated goldens:

- `tests/fixtures/target_gating/t1_import_http.diag`
- `tests/fixtures/target_gating/t1_import_sockets.diag`
- `tests/fixtures/target_gating/t1_import_udp.diag`

Current message shape:

`error[E0500|resolve]: module \`std::host::{http,sockets,udp}\` requires target wasm32-gc with host profile wasi-p2`

## Acceptance

- [x] Diagnostic parity reports no FAIL against committed goldens
- [x] `python3 scripts/manager.py selfhost diag-parity` exits 0
- [x] No new diagnostic parity mismatches introduced

## New-failure ratchet

No new diagnostic parity mismatches may be added. The count must only decrease.
