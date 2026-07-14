# CI required checks (quality closed loop)

Policy: [ADR-047](../adr/ADR-047-code-quality-tooling-and-gates.md).

## In-repo jobs

Workflow: [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

| Job | Command | Purpose |
|-----|---------|---------|
| `quality-format` | `python3 scripts/manager.py fmt --check` | Canonical Ark format |
| `quality-lint` | `python3 scripts/manager.py lint` | Correctness lint / smoke |
| `verify-quick` | `python3 scripts/manager.py verify quick` | PR-required quick verification |
| `verification` | `python3 scripts/manager.py verify` | Existing harness (includes `quality quick`) |
| `selfhost` | selfhost fixpoint / parity | Compiler bootstrap |
| `docs` | docs consistency | Docs hard gates |
| `verify` | aggregator | Needs the above |

Local and CI must call the same `manager.py` implementations.
EditorConfig, generated-output, SSOT, and boundary policy is reached through
`quality quick` / `quality structure`; workflow YAML does not copy those checks.

## GitHub ruleset contract

The `master` ruleset must require these status checks before merge:

1. `quality-format`
2. `quality-lint`
3. `verify-quick`
4. `Final gate` (aggregator)

The canonical API payload is
[`master-quality.json`](../../.github/rulesets/master-quality.json). Audit or
configure through repository Settings → Rules → Rulesets, or the GitHub API.
The status contexts must exactly match the Actions job names.

```bash
# Requires repository administration permission.
gh api repos/wogikaze/arukellt/rulesets \
  --method POST \
  --input .github/rulesets/master-quality.json
```

In-repo job presence is not evidence that the ruleset is active. Record the API
readback (ruleset id, enforcement state, target branch, required contexts) when
closing issue #790. Lack of admin permission leaves CQ-11 incomplete.

## CODEOWNERS

`.github/CODEOWNERS` assigns `src/compiler/`, `std/`, `docs/data/`, and
`scripts/gen/` to `@wogikaze`. Ruleset review requirements should use this
file rather than duplicating path ownership in workflow conditions.
