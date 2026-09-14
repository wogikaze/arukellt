# Local Cursor Issue Agent

This repository can use Cursor Agent CLI as a local GitHub Issue worker. The automation deliberately keeps reasoning/editing inside Cursor while branch, commit, push, PR, CI, and optional merge operations remain deterministic in `scripts/cursor_issue_agent.py`.

## Flow

`GitHub Issue -> local watcher/drain -> isolated git worktree -> Cursor Agent -> patch policy check -> commit/push -> PR -> CI -> optional merge`

The primary checkout is not switched or rewritten. Each Issue receives a temporary worktree based on the latest `origin/<default-branch>`.

## Prerequisites

Install and authenticate:

- Cursor CLI (`agent`; `cursor-agent` also works as the compatibility alias)
- GitHub CLI (`gh auth login`)
- Git
- Python 3

Cursor can use an existing local login or `CURSOR_API_KEY`. No Cursor credential is stored in the repository.

## Commands

Inspect the queue:

    python scripts/cursor_issue_agent.py list

Process one Issue, even if it was processed before:

    python scripts/cursor_issue_agent.py run --issue 123

Process every currently-open Issue that has not already been processed by this local state:

    python scripts/cursor_issue_agent.py drain

Process that backlog and squash-merge each PR only after its checks pass:

    python scripts/cursor_issue_agent.py drain --merge

Watch for Issues opened after the watcher starts:

    python scripts/cursor_issue_agent.py watch

Include the existing backlog, keep watching, and merge successful PRs:

    python scripts/cursor_issue_agent.py watch --backfill --merge

Useful options include `--model <cursor-model>`, `--interval 60`, `--check-interval 30`, `--base <branch>`, `--repo owner/name`, `--merge-method squash|merge|rebase`, and `--no-watch-ci`. `--merge` cannot be combined with `--no-watch-ci`.

## Queue semantics

A normal first `watch` marks the already-open Issues as seen and reacts only to newly opened Issues. `watch --backfill` and `drain` instead process open Issues that do not yet have a local processed-state record. `run --issue N` is the explicit retry/reprocess path.

State and generated PR-body files live under the repository's Git common directory at `.git/cursor-issue-agent/`; temporary worktrees live under the operating system temp directory. They are not repository content.

## Safety boundary

Issue text is treated as untrusted input. Cursor gets an agent-specific CLI config and command shims. It may use read-only `git`/`gh` inspection, but mutating Git/GitHub commands are blocked; the outer harness performs those operations after inspecting the changed-file list.

The harness refuses to publish patches that change its own prompt/config/script, repository agent instructions, Cursor rules, GitHub workflows/actions, or similar protected automation paths. Cursor is also denied `.env*` access. A failed Cursor run or a protected-path change is not automatically published; its worktree is preserved for inspection.

Automatic merging is opt-in with `--merge`. Without it, the worker stops after the PR checks finish and leaves the merge decision to a maintainer.

## Repository policy

`.github/cursor/issue-agent.json` can mark parent/mission Issue numbers or labels as non-closing. Generated PRs for those Issues use `Part of #N` rather than `Closes #N`, so an automated implementation step cannot accidentally close a long-running parent before its repository-defined completion gate passes.
