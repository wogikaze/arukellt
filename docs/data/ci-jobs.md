# CI jobs (generated)

> Generated from `.github/workflows/ci.yml` by `scripts/gen/generate-ci-jobs-doc.py`.
> Do not hand-edit the table. Unknown job IDs in current docs fail gate-765 / gate-769 checks.

| Job ID | Notes |
|--------|-------|
| `quality-format` | Canonical Ark formatter check through manager.py. |
| `quality-lint` | Ark correctness lint and lint-contract smoke through manager.py. |
| `verify-quick` | PR-required quick verification, including quality quick. |
| `verification` | Runs `python3 scripts/manager.py verify` (and related verify lanes). |
| `selfhost` | Selfhost fixpoint + fixture/CLI/diag parity (ADR-029). |
| `docs` | Docs consistency / freshness / structure checks. |
| `extension-tests` | VS Code extension activation and live CLI E2E. |
| `release-tag` | Tag-only: project-state.toml version vs git tag. |
| `verify` | Aggregator / final required-gate summary over blocking jobs. |
| `ci-category-summary` | Always-published category-to-job result summary. |

## Category mapping (informal)

| Category | Primary job ID |
|----------|----------------|
| verification / fixtures / docs checks | `verification`, `docs` |
| formatting / lint policy | `quality-format`, `quality-lint` |
| quick verification | `verify-quick` |
| bootstrap / selfhost parity | `selfhost` |
| editor / VS Code extension | `extension-tests` |
| release tag version | `release-tag` |
| merge aggregator | `verify` |
| run summary | `ci-category-summary` |

There is **no** `fixture-primary`, `verification-bootstrap`,
`verification-harness-quick`, or `determinism` top-level job in `ci.yml`.
Those historical names must not appear as current CI job IDs.
