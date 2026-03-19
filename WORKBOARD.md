# WORKBOARD

This file is the shared AI-managed task queue for the repository.
AI updates it; humans primarily read it.
It is the canonical place to park follow-up work, pick the next vertical slice, and record verified completion.

## Operating Rules

- Read this file before substantial planning or implementation work.
- Keep exactly one queue item in `Next`. If it becomes stale, promote the highest-priority unblocked item from `Ready`.
- Add newly discovered work to `Ready` unless a concrete dependency blocks it. Put dependency-gated work in `Blocked`.
- Keep task IDs stable as `WB-###`.
- Keep tasks as small vertical slices with one clear outcome.
- Move an item to `Done` only after the matching verification command or test has been run.
- When a task splits, add a follow-up item instead of mutating the old item beyond recognition.
- Keep `Done` entries concise and newest-first.
- Update this file in the same change when work starts, gets blocked, discovers follow-up tasks, or completes.

## Task Schema

Use this exact field order for every task:

### WB-000
title: Example task title
area: workflow
status: READY
priority: P2
owner: unassigned
depends_on: none
source: where this task came from
done_when:
- concrete verification outcome
notes:
- short context for future agents

Field rules:

- `status`: one of `NEXT`, `READY`, `BLOCKED`, `DONE`
- `priority`: one of `P0`, `P1`, `P2`, `P3`
- `owner`: `unassigned`, `ai`, or a short agent label
- `depends_on`: `none` or one or more `WB-###` identifiers
- `source`: file path, test name, user request, or other concrete origin
- `done_when`: 1 to 3 concrete checks
- `notes`: short bullets; newest note first if there are multiple notes

## Next

## Ready

## Blocked

### WB-008

title: Record a deployed GitHub Pages smoke URL for the docs shell
area: docs/release
status: BLOCKED
priority: P3
owner: unassigned
depends_on: repo-level GitHub Pages configuration
source: docs app shell exists, but deployment settings live outside the workspace
done_when:

- Pages source is configured
- the deployed URL is documented in the repo
- a smoke pass is recorded against the deployed site
notes:
- blocked on repository settings rather than code in this worktree

## Done
