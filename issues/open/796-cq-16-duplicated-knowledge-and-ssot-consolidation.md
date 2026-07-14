---
Status: open
Created: 2026-07-14
Updated: 2026-07-14
ID: 796
Track: code-quality
Depends on: "795"
Orchestration class: blocked
Orchestration upstream: 795
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-16
---

# 796 — CQ-16: duplicated knowledge and SSOT consolidation

## Summary

Classify duplicated compiler knowledge, select one existing owner for genuine
same-knowledge duplication, and add deterministic drift checks where views are derived.

## Scope

- Audit phase, target/profile, builtin/intrinsic, type spelling, opcode/tag,
  diagnostic, layout slot, WIT ABI, CLI, parser precedence, API export, and docs lists.
- Distinguish same knowledge, coincidental similarity, derived view, and
  compatibility spelling.
- Consolidate only genuine synchronization obligations.

## Non-goals

- No mega-registry, speculative global schema, or unification of coincidentally
  similar local logic.
- No new registry where an accepted SSOT already exists.

## Acceptance

- [ ] Every listed knowledge category has a recorded disposition
- [ ] Same-knowledge duplication has one owner
- [ ] Compatibility aliases are separated from canonical representation
- [ ] Derived views have deterministic generation and drift checks
- [ ] Existing registries are reused and local knowledge remains local
- [ ] Code, tests, docs, and generated views are synchronized
- [ ] Before/after owner inventory is recorded below

## Validation commands

- `python3 scripts/manager.py quality structure`
- `python3 scripts/manager.py docs regenerate`
- `python3 scripts/manager.py docs check`
- `python3 scripts/manager.py verify quick`
- Targeted mapping and generator tests named in completion evidence

## Completion evidence

Pending implementation and verification.

## Primary artifacts

- `docs/data/*.toml`
- `src/compiler/compiler/`
- `src/compiler/diagnostics/`
- `src/compiler/wasm/`
- `scripts/gen/`

## Remaining risks

- Compatibility spellings can look like accidental duplication.
- Moving subsystem-local facts to global data can increase coupling.

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
