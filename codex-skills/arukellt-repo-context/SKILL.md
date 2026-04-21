---
name: arukellt-repo-context
description: >-
  Use this skill when working in the Arukellt repository and you need the
  current source-of-truth files, verification contract, and markdown-reading
  workflow before implementing or reviewing changes.
---

# arukellt-repo-context

Use this skill at the start of Arukellt work when repo-specific operating rules matter.

## Primary Source Of Truth

Read these in order when relevant:

1. `docs/current-state.md` for current user-visible behavior
2. `issues/open/index.md` for the active work queue
3. `issues/open/dependency-graph.md` for dependency ordering
4. `issues/done/` for completed tracked work
5. `docs/adr/` for design rationale
6. `scripts/manager.py` for the verification contract
7. `scripts/gen/generate-docs.py` for generated docs behavior

## Markdown Reading

For large markdown files, prefer `markdive` over loading the whole file.

```bash
npx markdive dive <file> --depth 2
npx markdive dive <file> --path <section-id> --depth 2
npx markdive read <file> --path <section-id>
```

Use `dive` first, then narrow with `--path`, then `read` only the section you need.

## Verification

- Quick pass: `python scripts/manager.py verify quick`
- Full pass: `python scripts/manager.py verify`

If behavior changes and generated docs or issue indexes are affected, run:

```bash
python3 scripts/gen/generate-docs.py
python3 scripts/check/check-docs-consistency.py
bash scripts/gen/generate-issue-index.sh
```

## Tooling Notes

- Prefer `ig` for code search
- `ark-llvm` is present but excluded from default verification because it requires LLVM 18
- Generated docs and manifest-backed stdlib reference pages should be regenerated, not hand-maintained

## Bundled Skill Sources

Task-specialized Arukellt implementation skills are mirrored from `.github/agents/*.agent.md`
into `codex-skills/`. Install them into Codex discovery with:

```bash
bash scripts/util/install-codex-skills.sh
```
