# T3 emitter: WASI P2 import-table switch (full P2-native component)

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-22
**ID**: 510
**Depends on**: —
**Track**: wasi-feature
**Orchestration class**: implementation-ready
**Orchestration upstream**: #074-parent-gate
**Blocks v4 exit**: no

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

**Status note**: Leaf close-gate issue for #074. This unblocks the parent P2 native gate; it is not downstream of #074.

---

## Summary

The `--wasi-version p2` / `--p2-native` flags skip the WASI P1 adapter at the
component-wrapping stage (`crates/ark-wasm/src/component/wrap.rs`), but the
core Wasm module emitted by the T3 backend still imports the WASI Preview 1
interface (`wasi_snapshot_preview1`).  A fully working P2-native component
requires the T3 emitter to import P2 interface names directly.

This issue tracks the specific switch points identified during the 074
STOP_IF pass on 2026-04-15.

## Why deferred

Switching from P1 to P2 import names is **not** a simple string rename.  The
P1 ABI uses raw integer file-descriptor / iovec calling conventions, while
the P2 ABI uses the Component Model Canonical ABI (resource handles, string
lifting/lowering via `memory` and `realloc` imports).  Changing the import
names without adapting the call sites would generate invalid Wasm.

## Switch points found in the T3 emitter

### Primary: `crates/ark-wasm/src/emit/t3_wasm_gc/mod.rs`

All concrete WASI P1 imports are emitted in a single block starting at the
`ImportSection::new()` call (approximately line 1592).  Each import currently
uses `"wasi_snapshot_preview1"` as the module name.

| P1 import (module / name)                        | P2 equivalent interface                          |
|--------------------------------------------------|--------------------------------------------------|
| `wasi_snapshot_preview1` / `fd_write`            | `wasi:cli/stdout@0.2.0` / `write` (stream)       |
| `wasi_snapshot_preview1` / `path_open`           | `wasi:filesystem/types@0.2.0` / `open-at`        |
| `wasi_snapshot_preview1` / `fd_read`             | `wasi:cli/stdin@0.2.0` / `read` (stream)         |
| `wasi_snapshot_preview1` / `fd_close`            | `wasi:filesystem/types@0.2.0` / resource drop    |
| `wasi_snapshot_preview1` / `clock_time_get`      | `wasi:clocks/wall-clock@0.2.0` / `now`           |
| `wasi_snapshot_preview1` / `random_get`          | `wasi:random/random@0.2.0` / `get-random-bytes`  |
| `wasi_snapshot_preview1` / `proc_exit`           | `wasi:cli/exit@0.2.0` / `exit`                   |
| `wasi_snapshot_preview1` / `args_sizes_get` + `args_get` | `wasi:cli/environment@0.2.0` / `arguments` |
| `wasi_snapshot_preview1` / `environ_sizes_get` + `environ_get` | `wasi:cli/environment@0.2.0` / `environment` |

Each call-site also encodes the P1 ABI: linear-memory iovecs (`IOV_BASE`,
`IOV_LEN`, `NWRITTEN`), raw `u32` file descriptors, and scratch-pointer
passing.  Each must be replaced with the P2 Canonical ABI.

### Secondary: `crates/ark-target/src/plan.rs`

The `build_backend_plan` function at the `RuntimeModel::T1LinearP1 |
RuntimeModel::T3WasmGcP2` arm (approximately line 138) pushes an `ImportPlan`
with `module: "n_snapshot_preview1"` today.  When the T3 emitter gains the
P2 switch, this plan should branch on `WasiVersion`:

```rust
// TODO(510): branch on WasiVersion::P2 to emit P2 import plans
RuntimeModel::T3WasmGcP2 if wasi_version == WasiVersion::P2 => {
    // wasi:cli/stdout@0.2.0, wasi:cli/stdin@0.2.0, etc.
}
```

## Acceptance conditions for this issue

1. `crates/ark-wasm/src/emit/t3_wasm_gc/mod.rs` import block branches on a
   `p2_native: bool` parameter: P1 path unchanged, P2 path emits
   `wasi:cli/stdout@0.2.0` etc. with the Canonical ABI call convention.
2. `build_backend_plan` passes `WasiVersion` through to the `ImportPlan`
   builder.
3. A fixture compiled with `--target wasm32-wasi-p2 --wasi-version p2` and
   `--emit component` passes `wasm-tools validate` without errors.
4. Binary size is ≥ 80 KB smaller than the P1-adapter variant.
5. `python scripts/manager.py verify` passes.

## What was done in issue 074 (parent)

- `--wasi-version` CLI flag added (`crates/arukellt/src/main.rs`)
- `WasiVersion` enum added (`crates/ark-target/src/lib.rs`)
- `Session::wasi_version` field added (`crates/ark-driver/src/session.rs`)
- `--wasi-version p2` maps to `p2_native = true`, which skips the P1 adapter
  in `wrap_core_to_component` — the adapter bypass already works
- The T3 emitter P2 import-table switch is the remaining gap (this issue)
