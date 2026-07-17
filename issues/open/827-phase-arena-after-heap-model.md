---
Status: open
Created: 2026-07-17
Updated: 2026-07-17
ID: 827
Parent: 823
Track: selfhost-infra
Depends on: "730, 823"
Related: "#730, #823, #826, ADR-002"
Orchestration class: architecture-investigation
Blocks v4 exit: False
---

# P2b: phase arena (only after heap lifetime / ownership)

## Summary

Phase arenas may cut selfhost bump growth, but prototyping before ownership rules
are fixed risks leaking cross-phase refs into Wasm. **No arena product code until
the blockers below are decided** (ADR-002 / #730 connection).

## Prototype forbidden until decided

1. **Phase lifetime** — which phase owns which arena; when reset is legal
2. **Cross-arena references** — allowed graph (none / via durable handles only)
3. **Ownership of data that survives into final Wasm** — must not live in a
   resettable phase arena

## Acceptance

- [ ] Written decision (ADR update or linked design note) covering the three
      blockers above
- [ ] Explicit “no arena code before decision” remains true until that lands
- [ ] Only then: scoped prototype plan with reset points and verify gates
- [ ] Ties measurement to #823 wall/RSS receipts

## Non-goals (until decision)

- Any `Arena` / bump-reset implementation in `src/compiler/`
- Coupling arena work into #824 early body lowering

## References

- `issues/open/730-bootstrap-wasm-4gb-memory-limit.md`
- `issues/open/823-selfhost-compile-latency-quadratic-mir.md`
- `docs/research/selfhost-compile-latency-root-cause.md` (P2.3)
- ADR-002 (heap / allocation model — follow current ACCEPTED text)
