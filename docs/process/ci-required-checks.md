# CI required checks (quality closed loop)

Policy: [ADR-047](../adr/ADR-047-code-quality-tooling-and-gates.md).

## In-repo jobs

Primary workflow: [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

| Job | Command | Purpose |
|-----|---------|---------|
| `quality-format` | sharded `fmt --check` implementation; local equivalent is `python3 scripts/manager.py fmt --check` | Canonical Ark format |
| `quality-lint` | sharded `lint --local` implementation; local equivalent is `python3 scripts/manager.py lint` | Correctness lint / smoke |
| `verify-quick` | fast repository-contract checks in `ci.yml` | PR-required quick verification |
| `verification` | `python3 scripts/manager.py verify` | Existing harness (includes `quality quick`) |
| `selfhost` | selfhost fixpoint / parity | Compiler bootstrap |
| `docs` | docs consistency | Docs hard gates |
| `verify` | aggregator | Needs the above |

The following architecture jobs are separate workflows and are also required on
pull requests targeting `master`:

| Job | Command | Purpose |
|-----|---------|---------|
| `CoreHIR body boundary` | `python3 scripts/check/check-corehir-body-boundary.py` | Frozen CoreHIR body ownership contract |
| `MIR optimizer translation validation` | `python3 scripts/check/check-gc-hint-translation-validation.py` | Fail-closed MIR restoration and translation contract |
| `Typed proof contract frontend` | source/boundary checks and proof-adapter tests | Typed proof and architecture boundary contract |

Local commands and CI implementations must enforce the same contracts. The
sharded Ark jobs use the pinned selfhost wrapper for bounded PR feedback; the
`manager.py` commands remain the canonical local entry points.
EditorConfig, generated-output, SSOT, and boundary policy is reached through
`quality quick` / `quality structure`; workflow YAML does not copy those checks.

## GitHub ruleset contract

The `master` ruleset must require these status checks before merge:

1. `quality-format`
2. `quality-lint`
3. `verify-quick`
4. `CoreHIR body boundary`
5. `MIR optimizer translation validation`
6. `Typed proof contract frontend`
7. `Final gate` (aggregator)

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
