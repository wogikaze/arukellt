---
Status: open
Created: 2026-07-01
Updated: 2026-07-01
ID: 713
Track: language-docs
Depends on: "709, 711, 712"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#709 trait-first policy, #711 stdlib docs, #712 LLM quality signals"
Blocks v{N}: none
Priority: 2
Source: Stdlib best-practices docs direction 2026-07-01
---

# 713 — Stdlib and Arukellt code best-practices doc pack

## Summary

Arukellt needs a small, authoritative best-practices documentation pack that
LLMs and human contributors can read before writing code. The pack should be
compact enough to stay current: one to five Markdown files, each with clear
rules, examples, and links to the generated stdlib reference.

The goal is to prevent code that is technically accepted but signals poor API
design or poor readability.

## Proposed document set

Keep the set small. A likely shape is:

1. `docs/language/best-practices.md`
2. `docs/stdlib/best-practices.md`
3. `docs/stdlib/trait-first-api.md`
4. `docs/stdlib/collections-guide.md`
5. `docs/contributors/llm-code-review-signals.md`

The final filenames may differ, but the count should stay between one and five
Markdown files.

## Required work

- [ ] Document that high-level code should prefer traits, structs, `impl`
      methods, and associated functions over concrete helper functions.
- [ ] Document when low-level helpers are acceptable and when they are not.
- [ ] Document collection conventions: `Vec<T>`, `Deque<T>` or equivalent,
      hash collections, iterators, and conversion traits.
- [ ] Document readable expression style, including unary negative forms and
      avoiding workaround-looking expressions such as `0 - 1` where the
      language supports better syntax.
- [ ] Document error and absence modeling: prefer `Option` / `Result` over
      sentinel values where appropriate.
- [ ] Include before/after examples that are safe for LLM prompt context.
- [ ] Link the docs to generated stdlib reference pages.
- [ ] Add a docs drift check or review checklist so these docs stay aligned
      with #709 / #711 / #712.

## Acceptance

- [ ] One to five Markdown files cover Arukellt code best practices and stdlib
      API usage.
- [ ] The docs include concrete "avoid / prefer" examples.
- [ ] The docs explicitly discourage high-level use of monomorphic helpers
      once trait replacements exist.
- [ ] The docs link to generated stdlib pages for core traits and collections.
- [ ] The docs are referenced from contributor or LLM guidance.

## References

- #709 (trait-first API policy)
- #711 (rich stdlib reference docs)
- #712 (LLM code quality signal gates)
- `docs/current-state.md`
- `docs/stdlib/`
- `AGENTS.md`
