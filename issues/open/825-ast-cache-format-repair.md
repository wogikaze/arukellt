---
Status: open
Created: 2026-07-17
Updated: 2026-07-17
ID: 825
Parent: 823
Track: selfhost-infra
Depends on: "823"
Related: "#823, #824, docs/research/selfhost-compile-latency-root-cause.md"
Orchestration class: implementation
Blocks v4 exit: False
---

# AST cache format repair (not “re-enable as-is”)

## Summary

AST cache is effectively disabled (`AST cache disabled - needs heap investigation`).
This issue is **format/contract repair**, not a blind re-enable. Do not raise
implementation priority until frontend-time effect is measured against #823/#824.

## Acceptance

- [ ] Fix `deserialize_node` pos initialization bug
- [ ] Repair missing f64 handling in the cache codec
- [ ] Document binary I/O contract (endianness, lengths, node tags)
- [ ] Distinguish corrupt cache from empty AST
- [ ] Atomic write (temp + rename) for cache files
- [ ] Schema / version field; reject mismatched versions
- [ ] Revisit the 500-node limit policy (document keep / raise / remove)
- [ ] On cache hit: measure heap / peak RSS delta
- [ ] Measure frontend (lex/parse) wall vs cold before prioritizing over #824
- [ ] No product enablement that silently returns empty trees on corrupt input

## Non-goals

- Early body lowering (#824)
- Phase arena (#827)
- Raising priority above measured frontend benefit

## Notes

- Parent #823 explicitly deferred AST cache product work in the reachability slice.
- Prefer failing closed (miss / rebuild) over shipping a broken hit path.

## References

- `issues/open/823-selfhost-compile-latency-quadratic-mir.md`
- `docs/research/selfhost-compile-latency-root-cause.md` (原因6)
