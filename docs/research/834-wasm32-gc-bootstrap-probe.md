# #834 Probe: wasm32-gc self-emit on 23GiB host (2026-07-26)

## Host

- MemTotal ≈ 23 GiB, Swap 6 GiB (WSL2)
- Concurrent selfhost jobs present; lane used `ARUKELLT_BUILD_DIR=$PWD/.build`

## Phase 1 — emit without OOM

Default Memory64 s2-runtime (`initial_pages=65535`, ~4 GiB max linear):

| Attempt | Target | WASI | Wall | Max RSS | Result |
|---------|--------|------|------|---------|--------|
| default | wasm32-gc | wasi-p2 | 201.7s → later 61s cached | ~1.2–1.3 GiB | emit OK |
| m64 | wasm32-gc | wasi-p1 | 141.2s | ~1.2 GiB | emit OK |

**Conclusion:** Full selfhost `--target wasm32-gc` emit does **not** require OOM on this host
with the default 65535-page Memory64 runtime. Prior ~21 GiB RSS reports apply to
`--initial-pages≥98304` hosts, not this default path.

## Phase 2 — validate / typing

### wasi-p2 (memory32; `uses_memory64=false` by `#714`)

1. **Before import-index fix:** `func 160 cmd_dap` — `expected i64, found i32` at
   `call 4`. Cause: hardcoded P1 `fd_read` index 4 is P2 `open-at`.
2. **After import-index fix:** `func 8570 cmd_init` — `expected (ref null $type), found i32`
   around Result/match lowering for stubbed `fs::write_string` / init paths.
3. **After Result-local typing fix (fixture):** `$write_main` is `(ref null …)` and
   `write_init.ark` / `read_init.ark` validate under
   `wasm-tools validate --features gc,function-references,memory64`.

Artifact (pre-typing): `.build/selfhost/834-probe/selfhost-wasm32-gc-v2.wasm` (~5.5 MiB).

### wasi-p1 Memory64

`func 110 canonicalize_target_input` — GC String treated as linear i64 pointer
(`i32.wrap_i64` on a `(ref null string)`).

## Phase 2b — full self-emit regression on this host (later same day)

Re-emits of `src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2` after
`build-compiler` no longer finish reliably on this 23 GiB host:

| Invoke | Result |
|--------|--------|
| multi-dir (`--dir=.` + `src`/`std`/`.build`) | ~6 GiB RSS, flat CPU, no output (≥90s; killed) |
| single-dir / `_wasm_compile` | trap `out of bounds` at `0x100090000` (~4 GiB) in ~17s |
| clean `HEAD` s2 (no local typing edits) | same multi-dir hang |

So the hang/OOB is **not** uniquely caused by the Result-local patch; full pin emit is
blocked until the host emit path is stable again (quieter machine, Memory64 grow, or
dir/preopen setup matching the earlier 61s success).

## Usability blockers (beyond validate)

1. `emit_fs_read_to_string_gc` is a **typed Err stub** (not null) — pin still cannot read sources.
2. GC `write_string` / P2 `write_bytes` return **typed Ok stubs** — P2 has no file `fd_write`.
3. Full open/read GC unstub was attempted then deferred: fixture-sized modules validate, but
   full selfhost emit hung on this host even before that unstub landed.

## Next actions

1. Stabilize full `wasm32-gc`/`wasi-p2` self-emit on a quiet/large host; re-validate `cmd_init`.
2. Un-stub GC `fs::read_to_string` (heap path + open/read/close) once emit is stable.
3. Add P2 filesystem read/write imports if `BOOTSTRAP_EMIT` stays wasi-p2.
4. Then pin + flip `BOOTSTRAP_EMIT_*` + drop #813 stage-3 workaround + `verify quick`.
