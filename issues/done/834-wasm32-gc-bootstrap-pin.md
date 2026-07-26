---
Status: done
Created: 2026-07-25
Updated: 2026-07-26
ID: 834
Track: selfhost-infra
Depends on: "730"
Related: "#730, #827, #830"
Orchestration class: implementation-ready
Blocks v4 exit: True
---

# Pin bootstrap to validating Memory64 wasm32-gc

## Summary

Follow-up from [#730](730-bootstrap-wasm-4gb-memory-limit.md). Produce and pin a
validating **wasm32-gc / wasi-p2** bootstrap artifact (guest memory32) and green
`verify quick`. Note: guest memory is memory32 `(memory 8192)`; Memory64 applies
to the prior host s2-runtime emit path, not the pinned guest itself.

## Acceptance Criteria

- [x] stage-2 host compiles `src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2`
      to a module that `wasm-tools validate` accepts
- [x] Pinned bootstrap refreshed to that wasm32-gc / wasi-p2 artifact (memory32)
- [x] `BOOTSTRAP_EMIT_TARGET=wasm32-gc` / `BOOTSTRAP_EMIT_WASI_VERSION=wasi-p2` in
      [`scripts/selfhost/checks.py`](../../scripts/selfhost/checks.py)
- [x] Fixpoint stage-3 host restored to s2-runtime (drop #813 bootstrap-only workaround)
- [x] `python3 scripts/manager.py verify quick` passes (0 failures)

## Evidence

Probe receipt: [`docs/research/834-wasm32-gc-bootstrap-probe.md`](../../docs/research/834-wasm32-gc-bootstrap-probe.md)

| Gate | Evidence |
|------|----------|
| Emit+validate | GC guest validate PASS |
| Host-linker self-compile | flat-src `main.ark` PASS; then pin→s2→s3 fixpoint |
| Pin | `bootstrap/arukellt-selfhost.wasm` sha256 `4d2da710…` (= s2 = s3) |
| Emit flip | `BOOTSTRAP_EMIT_*=wasm32-gc/wasi-p2` |
| #813 drop | `_fixpoint_stage3_compiler` → `_stage3_compiler_wasm` (s2-runtime) |
| No `--to-memory64` on GC pin | `_ensure_bootstrap_compiler_wasm` / `_widen_compiler_wasm_to_memory64` |
| `verify quick` | 147/147 pass (0 fail, 0 skip) |

### Root-cause fixes that unblocked self-compile

1. GC `parse_f64` was a null-Result stub → `intrinsic_parse_f64_gc.ark`
2. `mir_int_literal_needs_i64` OOB via unguarded `|| char_at(..., 1)` → `literal_int.ark`
3. Earlier: vec-field `len`, nested `struct.get` stack base

## References

- [#730](730-bootstrap-wasm-4gb-memory-limit.md)
- `bootstrap/PROVENANCE.md`
- `docs/research/834-wasm32-gc-bootstrap-probe.md`
