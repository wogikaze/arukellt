---
Status: open
Created: 2026-07-13
Updated: 2026-07-13
ID: 791
Track: tooling-contract
Depends on: "785"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ-06 formatter full-repository audit
---

# 791 — Eliminate the Ark canonical parser baseline

## Summary

Remove every content-addressed fmt/lint parser exception from
`docs/data/ark-formatter-baseline.toml`. The current set contains compiler and
stdlib sources that the formatter parse gate rejects because of legacy source
forms, parser gaps, or source corruption. The baseline is a temporary migration
boundary, not permission to add more files.

## Acceptance

- [ ] Every listed file passes `python3 scripts/manager.py fmt --check <path>`
- [ ] Every listed compiler/stdlib file passes `python3 scripts/manager.py lint <path>`
- [ ] Every repaired file still compiles through the selfhost bootstrap path
- [ ] `docs/data/ark-formatter-baseline.toml` has zero exceptions and is removed
- [ ] `python3 scripts/manager.py quality full` passes without formatter skips

## Re-evaluation

Owner: compiler-tooling. Re-evaluate by 2026-08-31. An exception may be removed
only after format output reparses and the relevant compiler/stdlib verification
passes.
