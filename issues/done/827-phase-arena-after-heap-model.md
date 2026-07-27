---
Status: done
Created: 2026-07-17
Updated: 2026-07-25
Closed: 2026-07-25
ID: 827
Parent: 823
Track: selfhost-infra
Depends on: "730, 823"
Related: "#730, #823, #826, #834, ADR-002"
Orchestration class: done
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

- [x] Written decision covering the three blockers:
      [`docs/research/selfhost-phase-arena-ownership.md`](../../docs/research/selfhost-phase-arena-ownership.md)
      (2026-07-25; ADR-002 unchanged)
- [x] Explicit “no arena code before decision” remains true (no `src/compiler/**`
      Arena product code in this close)
- [x] Scoped prototype plan with reset points and verify gates (in the design note)
- [x] Ties measurement to #823 wall/RSS receipts (design note §計測との紐付け)

## Close note — 2026-07-25

Design-only close. Implementation requires a **new** open issue; do not add arena
product code from this issue. Upstream #823 done; #730 Memory64 / clone root-cause
narrow-closed (wasm32-gc pin continues in #834). `$issue-close-review`: **APPROVE**.

## Non-goals (until decision)

- Any `Arena` / bump-reset implementation in `src/compiler/`
- Coupling arena work into #824 early body lowering

## References

- `issues/open/730-bootstrap-wasm-4gb-memory-limit.md`
- `issues/open/823-selfhost-compile-latency-quadratic-mir.md`
- `docs/research/selfhost-compile-latency-root-cause.md` (P2.3)
- ADR-002 (heap / allocation model — follow current ACCEPTED text)
