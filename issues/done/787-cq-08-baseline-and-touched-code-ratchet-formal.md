---
Status: done
Created: 2026-07-13
Updated: 2026-07-13
ID: 787
Track: tooling-contract
Depends on: "786"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-08
---

# 787 — CQ-08: baseline and touched-code ratchet formalization

## Summary

Implement CQ-08 from the quality closed-loop plan.

## Acceptance

- [x] Deliverables for CQ-08 land and `python3 scripts/manager.py verify quick` passes
- [x] Primary artifact: `docs/data/ark-code-quality-baseline.toml`

## Completion evidence

- `docs/data/ark-code-quality-baseline.toml` names its owner and requires a tracking issue for updates.
- `scripts/check/check-ark-code-quality.py --changed` prevents touched-file regressions; baseline writes require `--issue`.
- `python3 scripts/manager.py verify quick`: 177/177 passed (2026-07-13).
- Re-audit 2026-07-14: legacy ceilings remain `lines_ge_200 = 437` and `thin_wrappers = 1733`; touched-code ratchet tests cover equal/pass and increase/fail, and `quality quick` passed.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
