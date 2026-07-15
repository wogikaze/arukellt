---
Status: open
Created: 2026-07-15
Updated: 2026-07-15
ID: 819
Parent: 729
Track: compiler-internal
Depends on: "727, 798"
Related: "676, 714, 818, ADR-007, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: blocked
Orchestration upstream: "727"
Blocks v{N}: none
Priority: 2
Source: ADR-042 runtime ABI migration ownership split
---

# 819 — Runtime ABI CoreOp lowering and emitter host-operation removal

## Summary

Move every runtime-classified CoreOp from `call_host_*` / `intrinsic_*` emitter
implementation to a declared runtime ABI or WIT import lowering. Consume #727
for HTTP/sockets standard WASI imports and cover the remaining host families here.

## Scope

- Inventory all `runtime_call` CoreOps and map each to `internal`, `wit`, or
  `native` runtime payload with an explicit ABI version.
- Remove emitter-owned implementations for stdio, fs, env, process, clock,
  random, HTTP, and sockets after their runtime/WIT path is executable.
- Keep public trait/method/associated APIs unchanged.
- Add differential or import-shape tests for every migrated runtime family.

## Non-goals

- Do not implement Ark stdlib operation bodies or the stdlib-only inliner.
- Do not change the CoreOp dispatch spine completed by #798.
- Do not duplicate #727's HTTP/sockets standard-WASI implementation; integrate it.

## Acceptance

- [ ] Every runtime-classified CoreOp resolves to a versioned runtime/WIT/native payload
- [ ] No runtime CoreOp dispatches to `call_host_*` or emitter `intrinsic_*` implementation
- [ ] HTTP/sockets use the standard WASI imports delivered by #727
- [ ] stdio/fs/env/process/clock/random runtime families have executable boundary tests
- [ ] Runtime import-shape and behavior differential tests pass
- [ ] `python3 scripts/manager.py verify quick` passes

## References

- `issues/open/727-arukellt-host-bridge-retirement.md`
- `issues/open/729-intrinsic-layer-separation.md`
- `issues/open/818-core-op-production-scaffold-exit.md`
- `data/core-ops.toml`
- `docs/adr/ADR-042-intrinsic-layer-separation.md`
