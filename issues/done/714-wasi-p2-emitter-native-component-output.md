---
Status: done
Status note: Bridged emitter-native WASI P2 path closed; verify quick green.
Created: 2026-07-02
Updated: 2026-07-25
Closed: 2026-07-25
ID: 714
Track: component-model
Parent: 668
Depends on: "074 (done), 510 (done)"
Orchestration class: architecture-implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: WASI 0.2 stdout architecture review 2026-07-02 — remove wrapper-only P2 component path
Close gate: scripts/check/gate-714-p2-emitter-native.py
---

# 714 — Emitter-native WASI P2 component output without wrapper

## Summary

Arukellt's P2 command output must be a valid Component Model artifact directly
from the compiler/emitter (bridged path), without `p2_component_wrap.py`.

## Acceptance

- [x] `wasm32-wasi-p2` / `wasm32-gc` + `wasi-p2` command compilation emits a
      Component Model binary directly; no post-compile `p2_component_wrap.py`
- [x] Generated P2 command components import `wasi:cli/stdout@0.2.0` and
      `wasi:io/streams@0.2.0` (component-level); no `::write` pseudo literal in
      the artifact. Guest core may still name a bridge `write` import (bridged
      short-term design); guest-native `get-stdout` call sites closed in #668
- [x] `tests/fixtures/wasi_p2_native/hello.ark` validates and prints `hello p2`
      under `wasmtime run`
- [x] Exit-code fixture `tests/fixtures/wasi_p2_native/exit_code.ark` proves the
      same emitter-native path (non-zero exit after `exit-marker` stdout)
- [x] `scripts/run/arukellt-selfhost.sh run --emit component` compiles to a
      component and runs via wasmtime without wrap
- [x] `p2_component_wrap.py` and related Python patch scripts deleted
- [x] `docs/current-state.md` / component-availability describe bridged
      emitter-native P2 output
- [x] #668 updated with bridged-close / guest-native remaining work
- [x] `python3 scripts/manager.py verify quick` exits 0

## Close gate

`python3 scripts/check/gate-714-p2-emitter-native.py` (PASS as of 2026-07-25):

1. In-tree component build for hello + exit_code
2. Fails if artifact contains `wasi:cli/stdout@0.2.0::write`
3. Fails if `p2_component_wrap.py` still exists
4. Requires `wasi:cli/stdout` + `wasi:io/streams` + `get-stdout`
5. `wasm-tools validate` + `wasmtime run` (`hello p2` / exit-marker + non-zero)

## Progress evidence — 2026-07-25 (bridged close)

| Step | Result |
|------|--------|
| Bridged emit ported to master-based `wave/714-p2-emitter-native` | OK |
| P2 path forces i32 memory (Memory64 off) for bridge share | OK |
| `gate-714-p2-emitter-native.py` | PASS |
| gate 074 / 510 | PASS |
| gate 076 | validate-only (runtime fs I/O remains #076) |
| Python wrap/patch scripts | deleted |

Closed 2026-07-25 after `verify lane` + `verify quick` (147/147) and `$issue-close-review` APPROVE.

## References

- `docs/plans` / research: `docs/research/p2-bridged-wasi-path-roadmap.md`
- `src/compiler/wasm/component_p2_emit.ark`, `component_p2_bridged.ark`
- `scripts/check/gate-714-p2-emitter-native.py`
- #668 (guest-native get-stdout / stderr polish)
- #727 (HTTP/sockets follow same architecture)


## Close note — 2026-07-25

**Verdict: APPROVE** (self-review against `docs/process/false-done-prevention.md`).

Evidence:
- `gate-714-p2-emitter-native.py` PASS
- gate 074 PASS (`hello p2`); gate 076 validate-only (runtime fs → #076)
- `p2_component_wrap.py` deleted; selfhost/runners have zero wrap refs
- Artifact has `wasi:cli/stdout` + `wasi:io/streams` + `get-stdout`; no `::write` literal
- Exit-code fixture proves emitter-native path
- `python3 scripts/manager.py verify quick` → 147 passed, 0 failed
- Guest-native get-stdout / stderr polish remains in #668; HTTP/sockets in #727
