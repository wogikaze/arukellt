---
Status: done
Created: 2026-07-17
Updated: 2026-07-26
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

- [x] Inventory of hot `clone` / string duplication sites with call-path notes
- [x] Proposed intern table ownership (which phase owns keys; lifetime)
- [x] At least one measured before/after on a bounded path (or explicit deferral
      if early body lowering must land first)
- [x] No arena code in this issue

## Close note (2026-07-26)

**Verdict: APPROVE** — investigation acceptance met; no arena code.

| Acceptance | Evidence |
|---|---|
| Inventory + call-path notes | [`docs/research/826-symbol-path-intern-clone-audit.md`](../../docs/research/826-symbol-path-intern-clone-audit.md) §1 |
| Intern table ownership / lifetime | same doc §2 (session-durable bump, `i32` handle) |
| Measured before/after | same doc §3.1; `src/compiler/collections/name_index.ark` (`find_slot` probe clone 除去). fair A/B wall 97.07s→76.20s (−21.5%), propagate 3023→2095 ms (−30.7%) |
| No arena | product diff is NameIndex only; #827 remains the arena ownership memo |

Landing commit: `d54cf4ad` (`fix(826): drop NameIndex probe clones; record intern audit.`).

Out of acceptance (not blocking close):

- KEEP_CLOCK `clone_calls` / `clone_bytes` 計装 — research deferral（計測は wall/propagate A/B で代替）
- callee predicate / Mir field id 化 — [#824](../open/824-early-body-lowering-worklist.md) および research §4 follow-up

## References

- `issues/done/823-selfhost-compile-latency-quadratic-mir.md`
- `docs/research/selfhost-compile-latency-root-cause.md`
- `docs/research/826-symbol-path-intern-clone-audit.md`
