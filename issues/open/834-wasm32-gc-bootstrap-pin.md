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

- **Not OOM-blocked** for default Memory64 s2-runtime (`initial_pages=65535`):
  full `wasm32-gc` self-emit finishes at ~1.2–1.3 GiB RSS on this 23 GiB host.
- Landed: P1/P2 WASI import-index helpers (`path_open` / `fd_read` / `stdin_read` /
  `fd_close`) so wasi-p2 stdin no longer calls `open-at`.
- Landed: wasi-p2 stub for GC `write_bytes` (no file `fd_write` on P2 core surface).
- **Still blocked for pin:**
  1. `wasm-tools validate` fails after index fix at `cmd_init` (GC Result/local typing).
  2. `emit_fs_read_to_string_gc` remains a null stub — pin would not read sources.
  3. Memory64 `wasm32-gc`/`wasi-p1` path fails validate in `canonicalize_target_input`
     (GC String used as linear i64 pointer).

## References

- [#730](730-bootstrap-wasm-4gb-memory-limit.md)
- `bootstrap/PROVENANCE.md` (wasm32-gc pinned blocked section)
- `docs/research/834-wasm32-gc-bootstrap-probe.md`
