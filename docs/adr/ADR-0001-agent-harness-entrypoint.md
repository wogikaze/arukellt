# ADR-0001: Agent Harness Entrypoint

- Status: Accepted
- Date: 2026-03-21

## Context

Arukellt uses agent-driven issue execution, but the repository did not have one explicit minimum-viable harness entrypoint that combined short pointer docs with deterministic completion checks. The queue and structural tests already existed, but the workflow was distributed across `AGENTS.md`, `README.md`, and issue files.

## Decision

Use a short pointer-style harness entrypoint in `docs/agent-harness.md`.

The entrypoint must:

- point agents to `issues/index.md` for queue state
- point agents to this ADR for the current decision record
- point agents to executable sources of truth such as tests and issue files
- require `./scripts/verify-harness.sh` before completion claims

`./scripts/verify-harness.sh` is the minimum deterministic local guardrail for repository hygiene. It runs formatter drift checks, a failing clippy gate for workspace libs and bins, workboard structure validation, and the full workspace test suite.

The clippy gate is currently scoped to `--lib --bins`. Test-target clippy cleanup is intentionally deferred until the repository is ready for that broader policy.

## Consequences

- Agent instructions stay short and point outward instead of duplicating mutable project state.
- Repository hygiene becomes verifiable through a stable command rather than prompt-only guidance.
- Future harness hardening should extend the script and tests incrementally instead of replacing this entrypoint with a larger bespoke system.
