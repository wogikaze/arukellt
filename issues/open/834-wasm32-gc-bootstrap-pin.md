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

- **Emit+validate on this 23 GiB host: PASS** via flat-src preopen
  (`--dir=.build/selfhost/flat-src --dir=.`, `-o bootstrap-out.wasm`, AST cache).
  Measured: WALL ≈ 100s, MAX_RSS ≈ 1.23 GiB, validate features
  `gc,function-references,memory64` OK (artifact
  `.build/selfhost/834-probe/selfhost-wasm32-gc-v7.wasm`).
- The earlier ~6 GiB “hang” / 4 GiB OOB was **wrong dir flags**, not hard OOM and
  not caused by the Result-typing patch.
- Landed: P1/P2 WASI import-index helpers; Result-local typing + typed GC fs stubs;
  `init.ark` no longer passes `String` into `fs_error_message` (FsError-only).
- **Still blocked for pin / close (do not false-done):**
  1. Runnable P2 host: smoke fails `unknown import wasi:cli/stdout@0.2.0::get-stdout`
     under plain wasmtime preview1-style run.
  2. GC `fs_read` still typed Err stub — cannot read sources.
  3. P2 has no file `fd_write`; write paths are typed Ok stubs.
  4. P2 read slot is `stdin.read`, not filesystem read.
  5. Memory64 `wasm32-gc`/`wasi-p1` path still fails validate in
     `canonicalize_target_input` (secondary).

### Close removal condition

P2 (or successor) filesystem read+write usable by bootstrap run +
`BOOTSTRAP_EMIT_*=wasm32-gc/wasi-p2` + drop #813 workaround + `verify quick` green.

## References

- [#730](730-bootstrap-wasm-4gb-memory-limit.md)
- `bootstrap/PROVENANCE.md` (wasm32-gc pinned blocked section)
- `docs/research/834-wasm32-gc-bootstrap-probe.md`
