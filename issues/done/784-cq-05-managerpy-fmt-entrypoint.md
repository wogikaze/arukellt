---
Status: done
Created: 2026-07-13
Updated: 2026-07-13
ID: 784
Track: tooling-contract
Depends on: "781"
Orchestration class: completed
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-05
---

# 784 — CQ-05: manager.py fmt entrypoint

## Summary

Implement CQ-05 from the quality closed-loop plan.

## Acceptance

- [x] Deliverables for CQ-05 land and `python3 scripts/manager.py verify quick` passes
- [x] Primary artifact: `scripts/manager.py`

## Completion evidence

- `scripts/manager.py fmt [--check] [paths...]` is the canonical formatter entrypoint.
- `scripts/tests/test_manager.py` covers command registration and dry-run routing.
- `python3 scripts/manager.py verify quick`: 177/177 passed (2026-07-13).
- Re-audit 2026-07-14: `manager.py fmt` and `fmt --check` parse; full check covered 1,981 files with zero failures and 26 exact-hash skips owned by #791.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
