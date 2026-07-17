---
Status: open
Created: 2026-07-17
Updated: 2026-07-17
ID: 824
Parent: 823
Track: selfhost-infra
Depends on: "823"
Related: "#730, #823, docs/research/selfhost-compile-latency-root-cause.md"
Orchestration class: design
Blocks v4 exit: False
---

# Early body lowering (worklist; design first)

## Summary

After MIR queue-BFS reachability (#823), full selfhost still lowers ~8737 MIR
bodies and keeps ~7980 after prune. Remaining latency is dominated by emitting
MIR for bodies that are never reachable from roots. Lower **bodies** on a
FunctionId worklist; register all signatures/layouts/types first.

## Design (acceptance for this issue = design lock + no premature impl)

```text
Register all signatures / FunctionIds / layouts / types
  → seed root FunctionIds (main / _start / exports / WIT / conservative set)
  → work queue
  → lower one function body
  → collect CALL / REF_FUNC / normal-fallback FunctionIds
  → enqueue → until empty
  → never-lowered bodies stay as signatures only
```

### Constraints (must appear in implementation plan)

1. **Do not delete CoreHIR declarations early.** Signature / FunctionId / layout /
   type metadata remain fully registered for the whole program.
2. **Body-only worklist.** Only function bodies are deferred; edges come from
   lowered CALL / REF_FUNC / normal-call fallback FunctionIds.
3. **Conservative keep.** If method / mono / closure / HOF / export / WIT roots
   cannot be proven safe to defer, **lower the body** (never silently drop).
4. **Separate state from MIR prune map.** Slice-1 `FunctionId → MirFunction index`
   is post-MIR. Early lowering needs `FunctionId → body-lowered?` at CoreHIR/MIR
   boundary — do not overload the MIR reachability index.
5. **Measurement gate.** Re-run the #823 receipt shape (wall / peak RSS /
   `reachability_fns` or equivalent body counts) after implementation.

## Non-goals

- AST cache repair (#825)
- Symbol/path interning (#826)
- Phase arena (#827)
- Changing public API / ABI / language semantics

## Acceptance

- [ ] Design section above remains the implementation contract
- [ ] Implementation plan lists root seeding and conservative-keep rules
- [ ] Implementation (follow-up commits under this issue) lands only after design
      review against #823 measurement conclusion
- [ ] `python3 scripts/manager.py verify quick` + selfhost build-compiler smoke
      when code lands

## Evidence / parent receipt

See #823 Notes: clock-stubbed s2-runtime full compile wall ~102 s, RSS ~1.32 GiB,
`lower.reachability_fns: before=8737 after=7980`. Conclusion: next CPU win is
skipping body lower for unreached FunctionIds.

## References

- `issues/open/823-selfhost-compile-latency-quadratic-mir.md`
- `docs/research/selfhost-compile-latency-root-cause.md` (原因5 / P1.1)
