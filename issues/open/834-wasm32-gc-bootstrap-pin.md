---
Status: open
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

Follow-up from [#730](730-bootstrap-wasm-4gb-memory-limit.md) after the
`clone(T)→T` MIR/emitter typing fix (`06ba2d35`). The known `func 8204`
`doc_parse_manifest` / `doc_flush_fn` ref-cast-to-String failure is addressed in
source; remaining work is producing and pinning a validating
**Memory64 `wasm32-gc` / `wasi-p2`** bootstrap artifact and greening
`verify quick`.

## Why this is a separate issue

Full selfhost `--target wasm32-gc` compile still needs a host that can grow past
the wasm32 4GiB linear ceiling. On a 23GiB WSL host, Memory64 hosts with
`--initial-pages≥98304` reach **~21GiB RSS** and are OOM-killed before emit
finishes; the default 65535-page s2-runtime still traps at
`0x1000…` (grow past 4GiB not effective in practice). This needs a quieter /
larger machine or a grow-path fix, not more clone typing.

## Acceptance Criteria

- [ ] stage-2 host compiles `src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2`
      to a module that `wasm-tools validate` accepts
- [ ] Pinned bootstrap refreshed to that Memory64 `wasm32-gc` / `wasi-p2` artifact
- [ ] `BOOTSTRAP_EMIT_TARGET=wasm32-gc` / `BOOTSTRAP_EMIT_WASI_VERSION=wasi-p2` in
      [`scripts/selfhost/checks.py`](../../scripts/selfhost/checks.py)
- [ ] Fixpoint stage-3 host restored to s2-runtime (drop #813 bootstrap-only workaround)
- [ ] `python3 scripts/manager.py verify quick` passes (0 failures)

## Evidence already landed (do not redo)

- Generic `clone` identity typing: `post_pass_callee_lookup.ark`,
  `code_ref_locals_infer_dest.ark`
- Fixture: `tests/fixtures/structs/struct_clone_pass_to_fn.ark` (t3-compile validate)

## Lane progress (2026-07-26, wave/834-bootstrap)

Probe receipt: [`docs/research/834-wasm32-gc-bootstrap-probe.md`](../../docs/research/834-wasm32-gc-bootstrap-probe.md)

### Done

- [x] Flat-src emit+validate `wasm32-gc`/`wasi-p2` selfhost (~1.23 GiB RSS; guest
      `(memory 8192)` memory32). Artifact tip: `selfhost-wasm32-gc-v17.wasm`.
- [x] P2 filesystem `read`/`write` imports + GC unstub + host-linker `p2_host.rs`.
- [x] Host smoke: `version` / `targets` / `fmt` / **compile hello** PASS.
- [x] GC emit fixes: `len(struct.vec_field)` type resolve; nested
      `a.b.c` / `get_unchecked(...).field` stack base (`inst_struct_record`).

### Still blocked for pin / close (do not false-done)

1. **GC guest cannot compile `src/compiler/main.ark`** — trap in
   `finish_decimal_number` while loading stdlib (`register_stdlib_source`).
   Pin + `BOOTSTRAP_EMIT_*` flip would break `build-compiler`.
2. Therefore: no pin refresh, no `BOOTSTRAP_EMIT_*` flip, no #813 drop, no close.

### Close removal condition

GC host-linker can self-compile `main.ark` → pin memory32 wasi-p2 artifact
(without `--to-memory64`) → flip `BOOTSTRAP_EMIT_*=wasm32-gc/wasi-p2` → drop #813
→ `verify quick` green.

## References

- [#730](730-bootstrap-wasm-4gb-memory-limit.md)
- `bootstrap/PROVENANCE.md` (wasm32-gc pinned blocked section)
- `docs/research/834-wasm32-gc-bootstrap-probe.md`
