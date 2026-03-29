# GitHub Copilot instructions

This repository is the Arukellt language toolchain: compiler/runtime backends, standard library, CLI, tests, and documentation.

## Primary source of truth

Prefer these files in this order when they are relevant:

1. `docs/current-state.md` for current user-visible behavior
2. `issues/open/index.md` for the active work queue
3. `issues/open/dependency-graph.md` for dependency ordering
4. `issues/done/` for completed tracked work
5. `docs/adr/` for design rationale
6. `scripts/verify-harness.sh` for the verification contract

## Markdown reading

When reading large Markdown files such as `README.md`, docs, ADRs, or issue indexes, do not load the whole file first.

Prefer `markdive` and use this workflow:

```bash
npx markdive dive <file> --depth 2
npx markdive dive <file> --path <section-id> --depth 2
npx markdive read <file> --path <section-id>
```

Use `dive` to inspect structure, `dive --path` to drill down, and `read` only for the section you actually need.

## Verification

Use:

- `bash scripts/verify-harness.sh --quick` for a quick pass
- `bash scripts/verify-harness.sh` for a full pass

## Tooling

- Prefer `ig` for code search.
- Regenerate generated docs instead of hand-editing them when behavior changes.
