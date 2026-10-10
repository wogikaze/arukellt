---
Status: done
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

## Resolution

`scripts/check/check-fresh-s2-smoke.py` runs 4 probes in parallel
against `.build/selfhost/arukellt-s2-runtime.wasm` via the wrapper with
a 420s hard per-probe timeout, validates every emitted module with
`wasm-tools validate`, and executes the assert-bearing linear probe
under wasmtime to catch silent mis-binds. Probe set: `generics`
`two_params` + `vec_generic` (wasm32-gc), plus inline dual-instantiation
programs on both targets (generic mono mint/bind). Wired as the
`s2-producer` verify-lane gate and into `verify quick` background
checks. Healthy run: 4/4 PASS in ~14s per probe.

Deviation from the suggested set: `std::seq` probes are deferred until
the s2 runtime is re-pinned - under the un-repinned s2 (old linear-GC
collect inside) a bare `use std::seq` compile sits in the quadratic mark
path for minutes, which would keep the smoke permanently red. The
deferral is documented in the script; the inline dual-instantiation
probes still cover the mono mint/bind path the historical regression
hit.

Gate efficacy was demonstrated against the stale Oct-9 s2 runtime: it
flagged the #908 mis-bind (gc probe failed validation, linear probe
trapped at run time).

Bisection while shaping the probes surfaced a previously undocumented
user-side repro of the same-named-generic mis-bind; filed as #914.
