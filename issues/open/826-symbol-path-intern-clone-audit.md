---
Status: open
Created: 2026-07-17
Updated: 2026-07-17
ID: 826
Parent: 823
Track: selfhost-infra
Depends on: "823"
Related: "#823, #730, #824, #827"
Orchestration class: investigation
Blocks v4 exit: False
---

# P2a: symbol / path interning + hot-path clone audit

## Summary

Reduce bump-heap pressure from repeated identifier/callee/path strings and
deep `clone` on hot selfhost compile paths. Independent of MIR reachability BFS
(#823) and early body lowering (#824).

## Scope

- Symbol / callee / path interning opportunities in CoreHIR → MIR → Wasm paths
- Audit deep `clone` on hot paths (sync, propagate, name maps, call edges)
- Prefer measured hot spots from #823 receipts over speculative wrappers

## Non-goals

- Phase arena prototyping (#827)
- AST cache repair (#825)
- Changing public API surface for interned strings unless ADR requires it

## Acceptance

- [ ] Inventory of hot `clone` / string duplication sites with call-path notes
- [ ] Proposed intern table ownership (which phase owns keys; lifetime)
- [ ] At least one measured before/after on a bounded path (or explicit deferral
      if early body lowering must land first)
- [ ] No arena code in this issue

## References

- `issues/open/823-selfhost-compile-latency-quadratic-mir.md`
- `docs/research/selfhost-compile-latency-root-cause.md` (P2.1–P2.2)
