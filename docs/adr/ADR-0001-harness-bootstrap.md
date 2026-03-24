# ADR-0001: Harness Bootstrap Strategy

**Status**: Accepted
**Date**: 2026-03-24
**Decided by**: Agent initialization

## Context

This project requires a structured harness to support:
- Agent-driven development workflows
- Clear completion criteria
- Reproducible local and CI environments
- Documentation governance
- Scalable task execution

The choice is between following a single reference implementation or synthesizing from proven patterns across multiple projects.

## Decision

Adopt a synthesized baseline harness combining:

1. **NEPLg2's explicit planning model** - `issues/open` queue with ADR-driven decisions
2. **vibe-lang's command discoverability** - Focused task surface with clear verification gates
3. **wado's environment discipline** - Bootstrap and artifact regeneration as first-class concerns

This is the "minimum viable harness" from `harness/blueprint.md`.

## Rationale

- **Clarity**: Short pointer docs and explicit queues make status obvious
- **Reproducibility**: Generated artifacts are committed with regeneration tasks
- **Scalability**: Foundation supports adding semantic tools, hooks, and skills later
- **Low ceremony**: Doesn't require full adoption of `mise` or `dprint` initially

## Implementation

The minimum viable harness includes:

- `AGENTS.md` - Repository contract and verification boundaries
- `docs/agent-harness.md` - Pointer document for developers
- `docs/adr/` - Workflow and design decisions
- `issues/open/` and `issues/done/` - Work queue
- `scripts/verify-harness.sh` - Root completion gate

Subsequent ADRs will address:
- Task runner selection (`mise` vs `just`)
- Formatter and lint stack
- CI pipeline structure
- Optional agent extension layers

## Consequences

- **Good**: Clear entry point for new contributors and agents
- **Good**: Completion criteria are explicitly defined
- **Good**: Can scale up incrementally without rework
- **Caution**: Requires discipline to keep `issues/open` and `issues/done` synchronized
- **Caution**: ADR archive must remain useful as decisions accumulate
