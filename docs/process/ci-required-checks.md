# CI required checks (quality closed loop)

Policy: [ADR-047](../adr/ADR-047-code-quality-tooling-and-gates.md).

## In-repo jobs

Workflow: [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

| Job | Command | Purpose |
|-----|---------|---------|
| `quality-format` | `python3 scripts/manager.py fmt --check` and editorconfig basics | Stage A format |
| `quality-lint` | `python3 scripts/manager.py lint` | Correctness lint / smoke |
| `verification` | `python3 scripts/manager.py verify` | Existing harness (includes quick quality checks) |
| `selfhost` | selfhost fixpoint / parity | Compiler bootstrap |
| `docs` | docs consistency | Docs hard gates |
| `verify` | aggregator | Needs the above |

Local and CI must call the same `manager.py` implementations.

## Recommended GitHub ruleset (manual)

Require status checks before merge to `master`:

1. `quality-format`
2. `quality-lint`
3. `verify` (aggregator) or `verification`

Configure via repository Settings → Rules → Rulesets, or:

```bash
# Example only — requires admin; adjust check names to match Actions.
gh api repos/{owner}/{repo}/rulesets --method POST --input ruleset.json
```

If the operator lacks admin rights, leave this as an open ops task on issue
# 790 and treat in-repo job presence as the CQ-11 acceptance bar.

## CODEOWNERS

Path ownership candidates live in
[`docs/process/codeowners-plan.md`](codeowners-plan.md). Do not commit an empty
`CODEOWNERS` file.
