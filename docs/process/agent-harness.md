# Agent Harness Guide

This is a short pointer document for developers and agents working in this repository.

## Quick Start

1. **Setup**: Follow `docs/process/agent-harness.md` for bootstrap instructions
2. **Tasks**: See `AGENTS.md` for repository contract and `scripts/run/verify-harness.sh` for verification
3. **Queue**: Check `issues/open/` for active work; see `issues/done/` for completed items
4. **Decisions**: Review `docs/adr/` for workflow and design decisions

## Verification Loop

```bash
scripts/run/verify-harness.sh
```

This is the fast local verification gate.
Before marking substantial work done, run the heavier groups you touched (for example `--cargo`, `--fixtures`) or use `scripts/run/verify-harness.sh --full`.

## Making Changes

1. Pick a task from `issues/open/`
2. Move it conceptually through: planning → implementation → verification
3. Update relevant docs/ADRs if approach changes
4. Run `scripts/run/verify-harness.sh` for a fast local verify pass; add `--cargo`, `--fixtures`, or `--full` when your change touches those surfaces
5. Move completed task to `issues/done/`

## Key Files

- **AGENTS.md** - Repository contract and boundaries
- **docs/adr/** - Architectural and workflow decisions
- **docs/process/agent-harness.md** - This document
- **issues/open/** - Active work queue
- **scripts/run/verify-harness.sh** - Root verification command

## If Something Breaks

1. Check `scripts/run/verify-harness.sh` output for specifics
2. Review recent ADRs in `docs/adr/` for context
3. Examine the queue in `issues/open/` for ongoing work
4. Refer to language-specific documentation for domain issues
