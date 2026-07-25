---
Status: open
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

## Lane progress (2026-07-26, `wave/826-intern-clone`)

正本: [`docs/research/826-symbol-path-intern-clone-audit.md`](../../docs/research/826-symbol-path-intern-clone-audit.md)

- Inventory: `post_pass_callee_lookup` (155) が最大。共通基盤は `NameIndex` probe clone。
- Ownership: session-durable intern table（phase arena reset 外）。`i32` handle。
- Measured win: `name_index_find_slot` の probe deep-clone 除去。
  fair A/B wall **97.07 s → 76.20 s (−21.5%)**, propagate **3023 → 2095 ms (−30.7%)**.
- Deferred: KEEP_CLOCK `clone_calls`/`clone_bytes` 計装; full Mir field id 化（#824 競合）。

## References

- `issues/done/823-selfhost-compile-latency-quadratic-mir.md`
- `docs/research/selfhost-compile-latency-root-cause.md`
- `docs/research/826-symbol-path-intern-clone-audit.md`
