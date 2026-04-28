---
name: arukellt-repo-context
description: >-
  Use when working in the Arukellt repository before implementing or reviewing
  changes. Triggers: need to find source-of-truth files, understand the
  verification contract, read large markdown files, or regenerate generated
  docs and issue indexes.
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

## Common Mistakes

| Mistake | Why It Happens | How to Avoid |
|---------|---------------|--------------|
| **Reading entire large markdown files** | "I need to see the full context" | Use `markdive` instead: `npx markdive dive <file> --depth 2` first, then drill down with `--path`. |
| **Hand-editing generated docs** | "It's faster to just fix this one line" | Generated docs should be regenerated, not hand-maintained. Modify the generator or manifest input instead. |
| **Trusting old roadmap prose** | "The README describes the architecture" | Use `docs/current-state.md` as the behavior contract, not roadmap prose or stale notes. |
| **Skipping verification** | "The change is too small to verify" | Always run `python scripts/manager.py verify quick` at minimum, even for small changes. |
| **Forgetting to regenerate indexes** | "I only moved one issue file" | Run `python3 scripts/gen/generate-issue-index.py` after any issue file move or status change. |

## Cross-References

- **REQUIRED:** Use this skill before any `impl-*`, `design-*`, `reviewer`, or `verify` skill to establish repo context.
- **IMPLEMENTATION:** After establishing context, use the relevant `impl-*` or `design-*` skill.
- **VERIFICATION:** Use `reviewer` for close review, then `verify` for issue closure.

## Verification

- Quick pass: `python scripts/manager.py verify quick`
- Full pass: `python scripts/manager.py verify`

If behavior changes and generated docs or issue indexes are affected, run:

```bash
python3 scripts/gen/generate-docs.py
python3 scripts/check/check-docs-consistency.py
python3 scripts/gen/generate-issue-index.py
```

## Tooling Notes

- Prefer `ig` for code search
- Generated docs and manifest-backed stdlib reference pages should be regenerated, not hand-maintained
