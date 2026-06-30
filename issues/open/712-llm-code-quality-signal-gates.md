---
Status: open
Created: 2026-07-01
Updated: 2026-07-01
ID: 712
Track: tooling-contract
Depends on: "709, 711"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: LLM readability signal direction 2026-07-01
---

# 712 — LLM code quality signal gates for readability and stdlib misuse

## Summary

The compiler, linter, docs, and verification tooling should make it easy for
LLMs and humans to notice when generated Arukellt code is drifting toward poor
style. Patterns such as `return 0 - 1` in place of `return -1`, high-level code
calling `*_i32` helpers, or manually reimplementing collection behavior are
signals that the language or stdlib surface is not teaching the right shape.

This issue tracks machine-checkable warnings, docs, and review gates that
report these signals early.

## Current state

- Some code readability issues are visible only during review.
- The stdlib still exposes helper-style APIs that LLMs can overuse.
- There is no documented checklist for "LLM generated code looks suspicious"
  patterns in Arukellt.

## Required work

- [ ] Define a short catalog of readability and stdlib misuse signals:
      - `0 - 1` where unary negative literal syntax is intended
      - public/high-level use of `*_i32` helpers when trait APIs exist
      - manual loops replacing available iterator adapters
      - manual collection wrappers replacing public collection structs
      - sentinel return values where `Option` / `Result` should be used
- [ ] Decide which signals belong in compiler diagnostics, lints, docs checks,
      or review checklist only.
- [ ] Add fixtures for accepted and rejected examples.
- [ ] Add a gate or linter mode that can run in `verify quick` once stable.
- [ ] Update docs so LLMs receive the preferred form before examples of the
      low-level form.
- [ ] Ensure each warning points to the trait / struct / module replacement.

## Acceptance

- [ ] There is a documented signal catalog for LLM-readable Arukellt code.
- [ ] At least one automated gate detects a representative readability issue.
- [ ] At least one automated gate detects high-level use of deprecated or
      low-level stdlib helpers after replacements exist.
- [ ] Diagnostics or docs explain the preferred replacement form.
- [ ] The gate is wired into the repo verification contract when stable.

## References

- #709 (trait-first API policy)
- #711 (rich stdlib docs)
- `src/compiler/lint/`
- `docs/current-state.md`
- `scripts/manager.py`
