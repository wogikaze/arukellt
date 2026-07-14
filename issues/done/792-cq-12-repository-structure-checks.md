---
Status: done
Created: 2026-07-14
Updated: 2026-07-14
ID: 792
Track: tooling-contract
Depends on: "782, 790"
Orchestration class: completed
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: CQ closed-loop plan CQ-12
---

# 792 — CQ-12: repository structure checks

## Summary

Provide one canonical, machine-readable entrypoint for hard repository and
compiler structure contracts without promoting size or complexity metrics to
absolute gates.

## Acceptance

- [x] `python3 scripts/manager.py quality structure` reports hard structure contracts
- [x] `quality structure --json` emits schema version 1 from the same finding model
- [x] Import cycles, reverse pipeline dependencies, production-to-test reachability, and public-boundary bypasses fail
- [x] Generated views, SSOT registries, commands, CI jobs, and documentation contracts are checked
- [x] Size, complexity, nesting, fan-out, long-line, and thin-wrapper values remain advisory or existing ratchets
- [x] Normal and violation fixtures cover the canonical structure collector
- [x] `quality quick` and `verify quick` execute the hard structure collector

## Completion evidence

- `scripts/quality/structure.py` owns the schema-versioned finding model and calls existing generated, repository, and compiler-boundary checkers.
- `docs/data/code-quality-rules.toml` registers `CQ-STRUCT-002` through `CQ-STRUCT-009`; `docs/data/verification-commands.toml` registers `quality_structure`.
- `scripts/tests/test_quality_structure.py` covers cycle, dependency direction, test-only reachability, duplicate IDs, missing commands/jobs, and text/JSON consistency.
- `python3 scripts/manager.py quality structure` and `--json`: pass with zero findings (2026-07-14).
- `python3 scripts/manager.py quality quick`, `quality full`, and `verify quick`: exit 0 (2026-07-14).

## Primary artifacts

- `scripts/quality/structure.py`
- `docs/data/code-quality-rules.toml`

## References

- `docs/adr/ADR-047-code-quality-tooling-and-gates.md`
- `docs/adr/ADR-048-design-heuristics-application-order.md`
