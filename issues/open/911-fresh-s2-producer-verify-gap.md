---
Status: open
Created: 2026-10-10
Updated: 2026-10-10
ID: 911
Track: verification
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: f678e96d8 regression masked ~2 weeks - fresh s2 never runs as producer
---

# 911 - Verification gap: nothing exercises a freshly built s2 as producer

## Summary

scripts/run/arukellt-selfhost.sh prefers the pinned bootstrap wasm when
it exists, so every day-to-day compile/check runs the OLD compiler. The
only lane that runs a freshly built s2 as the producer is
selfhost fixpoint --build, which takes ~25 min and is not run in the
normal edit loop. A regression that makes s2 hang or mis-emit as a
producer therefore ships undetected: f678e96d8 (2026-09-29) left
check/compile of any `use std::seq` program hanging forever under fresh
s2 while the pinned bootstrap compiled the same file in ~1s.

## Suggested direction

After selfhost build-compiler, run a bounded smoke set (3-5 probes
covering generic monomorphization through std wrappers, std::seq use,
and emit of both wasm32-gc and wasm32 outputs) directly against
.build/selfhost/arukellt-s2-runtime.wasm with a hard timeout. Cheap
enough to join verify lane.
