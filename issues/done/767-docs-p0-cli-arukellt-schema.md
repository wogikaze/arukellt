---
Status: done
Created: 2026-07-11
Updated: 2026-07-11
ID: 767
Track: docs-audit
Depends on: 765
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Docs re-audit 2026-07-11 (P0-2)
Blocks: 770
---

# 767 — Docs P0: CLI surface `arukellt` single schema

## Summary

`cli-reference.md` documents `ark doc` / `ark component` while the binary and
usage string are `arukellt`. `ark-toml.md` still mentions `ark build`.
`component` / `compose` exist in selfhost CLI but must be spelled consistently
with `current-state.md` and quickstart.

## Acceptance

- [x] All current CLI docs use binary name `arukellt` (no undefined `ark` alias)
- [x] `cli-reference.md` matches usage.ark subcommands (`component`, `compose`, `doc`, …)
- [x] `current-state.md` CLI table includes `component` / documents emit relationship
- [x] `ark-toml.md` project commands use `arukellt build`
- [x] Docs-related verify gates pass

## References

- `src/compiler/main/usage.ark`
- `src/compiler/main/component_cmd.ark`
- `docs/current-state.md` CLI section


## Completion

Completed 2026-07-11 as docs re-audit Phase 1.
