---
Status: done
Created: 2026-07-13
Updated: 2026-07-14
ID: 790
Track: tooling-contract
Depends on: "784, 786, 787"
Orchestration class: completed
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-11
---

# 790 — CQ-11: CI quality-format/lint jobs and required-checks docs

## Summary

Implement CQ-11 from the quality closed-loop plan.

## Acceptance

- [x] Deliverables for CQ-11 land and `python3 scripts/manager.py verify quick` passes
- [x] Primary artifact: `.github/workflows/ci.yml`

## Completion evidence

- `.github/workflows/ci.yml` defines independent `quality-format`,
  `quality-lint`, and `verify-quick` jobs. `Final gate` depends on all three,
  and the category summary identifies each responsible job.
- `.github/rulesets/master-quality.json` is the versioned ruleset contract.
  GitHub API readback on 2026-07-14 confirmed ruleset `18894318` (`master
  quality gates`) is active for the default branch with no bypass actors.
- The active ruleset requires `quality-format`, `quality-lint`,
  `verify-quick`, and `Final gate`. It also requires one approval, stale
  review dismissal, CODEOWNER approval, and review-thread resolution.
- `.github/CODEOWNERS` assigns the compiler, stdlib, structured docs data,
  and documentation generators to `@wogikaze`.
- `python3 scripts/manager.py verify quick`: 177 passed, 0 skipped, 0 failed.
- `python3 scripts/check/check-false-done-close-gates.py`: 28 enforced gates
  passed when run outside the socket-restricted sandbox.
- Re-audit 2026-07-14: CI YAML parsed with independent `quality-format`,
  `quality-lint`, `verify-quick`, and aggregator jobs. The quality jobs invoke
  only `manager.py fmt --check` / `manager.py lint` for quality policy.
- GitHub API readback re-confirmed ruleset `18894318` active, no bypass actors,
  with required contexts `quality-format`, `quality-lint`, `verify-quick`, and
  `Final gate`.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
