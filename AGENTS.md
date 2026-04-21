# AGENTS.md

This repository is the **Arukellt** language toolchain: compiler/runtime backends, standard library, CLI, tests, and documentation.

## Repository Boundary

This repo contains:

- compiler and runtime implementation under `crates/`
- standard library sources under `std/`
- CLI integration in `crates/arukellt`
- fixture / benchmark / verification infrastructure under `tests/`, `benchmarks/`, and `scripts/`
- user-facing and design documentation under `docs/`

## Primary Source of Truth

Use these in order, depending on the question:

- **Current user-visible behavior**: `docs/current-state.md`
- **Current open work queue**: `issues/open/index.md`
- **Current dependency ordering**: `issues/open/dependency-graph.md`
- **Completed tracked work**: `issues/done/`
- **Design decisions / rationale**: `docs/adr/`
- **Verification contract**: `scripts/manager.py` (`python scripts/manager.py verify`)
- **Generated docs contract**: `scripts/gen/generate-docs.py`

## Current Work Surface

The active open queue is the generated issue index under `issues/open/`.
At the time of writing, the queue is centered on:

- WASI Preview 2 native component output
- `std::host::*` namespace rollout
- shared host capability facades across T1 / T3

Do not infer active work from old roadmap prose when `issues/open/index.md` disagrees.

## Documentation Rules

- Treat `docs/current-state.md` as the current behavior contract.
- Many landing pages are generated. After changing manual doc sources, regenerate docs with:

```bash
python3 scripts/gen/generate-docs.py
```

- Check for docs drift with:

```bash
python3 scripts/check/check-docs-consistency.py
```

- If queue structure changes, regenerate issue indexes with:

```bash
python3 scripts/gen/generate-issue-index.py
```

## Completion Criteria

Work is complete when the relevant scope is updated and verification passes.
For tracked issue work, that normally means:

1. `python scripts/manager.py verify` exits with status 0
2. generated artifacts touched by the work are regenerated and included
3. relevant docs / ADRs are updated when behavior changed
4. tracked issue files move from `issues/open/` to `issues/done/` when the task itself is completed
5. commits stay focused to the files changed for that task

## Verification Loop

- Quick pass: `python scripts/manager.py verify quick`
- Full pass: `python scripts/manager.py verify --full`

## Tooling Notes

- Prefer `ig` for code search.
- `ark-llvm` is present in the workspace but excluded from normal default verification because it requires LLVM 18.
- Generated docs and manifest-backed stdlib reference pages should be regenerated, not hand-maintained.

## Codex Skills

- Repo-local Codex skill sources live under `codex-skills/`.
- These are mirrored from `.github/agents/*.agent.md` plus shared repo context guidance.
- The `scripts/util/install-codex-skills.sh` installer was removed; copy or symlink the `codex-skills/*` directories into `$CODEX_HOME/skills` (or `~/.codex/skills`) manually if needed.

## Markdown Navigation

- When reading large Markdown files such as `README.md`, docs, ADRs, or issue indexes, prefer `markdive` over loading the whole file at once.
- Use `npx markdive` so the workflow works even when the CLI is not globally installed.
- Recommended flow:

```bash
npx markdive dive <file> --depth 2
npx markdive dive <file> --path <section-id> --depth 2
npx markdive read <file> --path <section-id>
```

- First inspect structure with `dive`, then drill down with `--path`, and only then read the target section with `read`.
- Fall back to normal file reads only when `markdive` cannot handle the document.
