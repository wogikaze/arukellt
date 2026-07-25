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

## Phase 2 — validate still fails

### wasi-p2 (memory32; `uses_memory64=false` by `#714`)

1. **Before import-index fix:** `func 160 cmd_dap` — `expected i64, found i32` at
   `call 4`. Cause: hardcoded P1 `fd_read` index 4 is P2 `open-at`.
2. **After import-index fix:** `func 8570 cmd_init` — `expected (ref null $type), found i32`
   around Result/match lowering for stubbed `fs::write_string` / init paths.

Artifact: `.build/selfhost/834-probe/selfhost-wasm32-gc-v2.wasm` (~5.5 MiB).

### wasi-p1 Memory64

`func 110 canonicalize_target_input` — GC String treated as linear i64 pointer
(`i32.wrap_i64` on a `(ref null string)`).

## Usability blockers (beyond validate)

1. `emit_fs_read_to_string_gc` still returns null Result — a wasm32-gc pin cannot read sources.
2. P2 core import surface has open-at/close/stdin.read but **no file fd_write**;
   `write_bytes_gc` is now stubbed under wasi-p2 so validate is not blocked by fd_write ABI.

## Next actions

1. Fix `cmd_init` / GC Result local typing after write stubs (or restore real GC write/read).
2. Un-stub GC `fs::read_to_string` with GC String→heap + WASI open/read (P1 first).
3. Add P2 filesystem read/write imports if BOOTSTRAP_EMIT stays wasi-p2.
4. Then pin + flip `BOOTSTRAP_EMIT_*` + drop #813 stage-3 workaround + `verify quick`.
