# PR / agent design review checklist

Policy: [ADR-048](../adr/ADR-048-design-heuristics-application-order.md),
[ADR-047](../adr/ADR-047-code-quality-tooling-and-gates.md).

Do **not** re-check formatter or linter items here. Run
`python3 scripts/manager.py fmt --check` and
`python3 scripts/manager.py lint` (or pre-commit) first.

## Design order (fixed)

1. Are the behavior and contract correct for the current requirement?
2. Is this the simplest direct implementation (KISS)?
3. Is there speculative abstraction for uncertain future requirements (YAGNI)?
4. Is the owner of data and responsibility unique?
5. Is duplication the same knowledge, or accidental similarity?
6. If the same knowledge, apply DRY to a single authority.
7. If change reasons differ, apply local SOLID / split only then.
8. Are there extension points or interfaces without a second real example?
9. Is public surface widened without need?
10. Are errors, side effects, rollback, and compatibility clear?
11. Do tests prove the contract, not the current implementation shape?
12. Do comments preserve reasons the code cannot express?
13. Are docs, manifest, generated output, and issues in sync?

Items 1–8 match ADR-048 steps 1–8; 9–13 are review-only follow-through
(ADR-048 step 9 covers comments/ADR for non-code constraints).

Forbidden review comments: "violates SOLID", "not DRY" without naming the
concrete sync risk, mixed responsibility, or dependency problem.
