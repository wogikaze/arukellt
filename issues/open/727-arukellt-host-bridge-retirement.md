---
Status: open
Created: 2026-07-10
Updated: 2026-07-10
ID: 727
Track: wasi-feature
Depends on: "714, 675"
Orchestration class: architecture-implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: ADR-007 (2026-07 revision) policy audit — arukellt_host custom bridge contradicts WASI P2/P3 unification
---

# 727 — Retire `arukellt_host` custom host bridge; migrate HTTP/sockets to standard WASI P2/P3 imports

## Summary

ADR-007 (2026-07 revision) abolished the custom `arukellt_io` host module and
mandated that **all host functions go through standard WASI P2/P3 imports**.
However, `tools/host-linker` still provides a non-standard `arukellt_host`
Wasmtime linker module (`host_http.rs`, `host_sockets.rs`) that bridges
`std::host::http` and `std::host::sockets` via private import functions
(`__intrinsic_http_get`, `__intrinsic_sockets_connect`, etc.).

This `arukellt_host` bridge is a **legacy layer that contradicts ADR-007**.
It must be retired in favor of standard WASI 0.2 interface imports:

- `wasi:http/outgoing-handler@0.2.x` for HTTP client
- `wasi:http/incoming-handler@0.2.x` for HTTP server
- `wasi:io/sockets@0.2.x` (or `wasi:sockets/tcp@0.2.x`) for TCP sockets

The compiler emitter should generate component-correct WIT imports for
these capabilities, and `tools/host-linker` should link standard WASI
imports (via `wasmtime-wasi` with `wasi-http` feature) instead of the
custom `arukellt_host` module.

## Problem

The current architecture is inverted, mirroring the same pattern #714
identified for stdio:

1. **Compiler** (`src/compiler/wasm/sections_imports.ark`,
   `import_indices.ark`, `emit_target.ark`) emits `arukellt_host::*`
   core imports — a private, non-standard module name.
2. **Stdlib** (`std/host/http.ark`, `std/host/sockets.ark`) calls
   `__intrinsic_http_*` / `__intrinsic_sockets_*` which lower to
   `arukellt_host` imports.
3. **Runtime** (`tools/host-linker/src/host_http.rs`, `host_sockets.rs`)
   implements HTTP/sockets in Rust and registers them as a custom
   `arukellt_host` Wasmtime linker module.

This means HTTP/sockets capability is **not portable** — it only works
with `arukellt-host-run`, not with standard `wasmtime run` or
`jco transpile`. ADR-007's goal is that all host functions use standard
WASI imports so any compliant runtime can execute the component.

## Goals

1. **Migrate `std::host::http`** from `arukellt_host::http_*` intrinsics
   to standard `wasi:http/outgoing-handler@0.2.x` component imports.
2. **Migrate `std::host::sockets`** from `arukellt_host::sockets_*`
   intrinsics to standard `wasi:io/sockets@0.2.x` (or equivalent)
   component imports.
3. **Update the emitter** to generate WIT-shaped component imports for
   HTTP and sockets, following the same architecture as #714's stdio
   path (canonical ABI glue, resource handles, lowered imports).
4. **Update `tools/host-linker`** to link standard `wasmtime-wasi`
   HTTP/sockets implementations instead of the custom `arukellt_host`
   module. The `host_http.rs` and `host_sockets.rs` files should be
   removed or reduced to thin shims that delegate to `wasmtime-wasi`.
5. **Remove `arukellt_host` import surface** from the compiler
   (`sections_imports.ark`, `import_indices.ark`, `emit_target.ark`,
   `function_indices.ark`, etc.) once stdlib no longer references it.
6. **Keep `std::host::http` / `std::host::sockets` public API stable**
   — the user-facing facade (ADR-011) does not change; only the
   backend bridge changes.

## Non-goals

- Changing the `std::host::http` / `std::host::sockets` user-facing API
  surface (ADR-011 facade stays).
- Full HTTP/2, HTTPS, or TLS support (separate capability work).
- UDP capability migration (`std::host::udp` is not yet user-reachable;
  tracked by #675).
- Debug adapter changes in `tools/host-linker` (debug support is
  orthogonal to the host import bridge).
- T4 native or LLVM backend changes.

## Acceptance

- [ ] `std::host::http::get` and `::request` compile to component imports
      referencing `wasi:http/outgoing-handler@0.2.x`, not
      `arukellt_host::http_*` core imports
- [ ] `std::host::http::serve` compiles to `wasi:http/incoming-handler`
      component imports (or documented equivalent)
- [ ] `std::host::sockets::connect` / `read` / `write` / `listen` /
      `accept` compile to `wasi:io/sockets@0.2.x` (or equivalent)
      component imports, not `arukellt_host::sockets_*`
- [ ] `tools/host-linker` links HTTP/sockets via `wasmtime-wasi`
      (with `wasi-http` feature) instead of custom `host_http.rs` /
      `host_sockets.rs` implementations
- [ ] `arukellt_host` module name no longer appears in compiler import
      sections or `tools/host-linker` linker registration for HTTP/sockets
- [ ] Existing gate fixtures pass with the new import path:
      `gate-655-http-outgoing.py`, `gate-656-http-incoming.py`,
      `gate-657-sockets-connect-read-write.py`,
      `gate-658-sockets-listen-accept.py`
- [ ] `wasm-tools validate` passes on HTTP/sockets fixture components
- [ ] `wasmtime run` (with `--wasm-features` / `--wasi` flags as needed)
      executes HTTP/sockets fixtures without the custom `arukellt_host`
      linker
- [ ] `docs/current-state.md` and `docs/capability-surface.md` updated
      to reflect standard WASI P2 import path for HTTP/sockets
- [ ] `std/manifest.toml` updated: HTTP/sockets bridge description
      no longer references `arukellt_host`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Close gate

Add or extend a gate under `scripts/check/` that:

1. Compiles an HTTP fixture and asserts the produced component imports
   `wasi:http/outgoing-handler@0.2.x` (not `arukellt_host::http_*`).
2. Compiles a sockets fixture and asserts the produced component imports
   `wasi:io/sockets@0.2.x` (not `arukellt_host::sockets_*`).
3. Fails if `arukellt_host` appears in the import section of any
   HTTP/sockets fixture component.
4. Runs the fixtures under `wasmtime run` without custom host linking.

## Dependency Notes

- Depends on **#714** (emitter-native P2 component output) — the
  canonical ABI glue and component emission infrastructure built for
  stdio must be in place before HTTP/sockets can follow the same path.
- Depends on **#675** (host capability user-reachability) — permission
  flags and manifest honesty must be reconciled so the migrated
  capabilities remain user-reachable.
- Related: **#668** (P2 native component polish) — this issue extends
  the wrapper-free architecture from stdio to HTTP/sockets.
- Related: **#676** (std::host fs/env/process) — fs/env/process already
  use standard WASI P2 imports (not `arukellt_host`); this issue brings
  HTTP/sockets to the same standard.

## Related: retire `wasm-heap-grow-patcher` (walrus dependency)

The workspace has a second external Rust dependency in the selfhost
bootstrap pipeline: `scripts/bootstrap/wasm-heap-grow-patcher` (depends
on `walrus` crate).  This tool post-processes the pinned wasm before
stage-2 compilation.  It should be retired alongside the host bridge —
both are external Rust dependencies that the selfhost pipeline should
not need.

### What the patcher does

1. **Memory expansion**: bumps `initial` to 65536 pages (4 GiB) and
   removes `maximum`.  The pinned wasm ships with 128 pages (8 MiB);
   the current selfhost compiler emits 8192 pages (512 MiB) for non-GC
   targets (`sections_memory.ark` → `initial_memory_pages()`).
2. **Vec_new overflow guard**: replaces the bump allocator prologue in
   `Vec_new` intrinsics with a u32-wraparound-aware version.
3. **Export deduplication**: removes duplicate export names (first-wins).

A fourth post-processing step (`_patch_bootstrap_disable_selfhost_mir_prune`
in `scripts/selfhost/checks.py`) flips a `prune=1` flag to `prune=0` in
the pinned wasm binary so stage-2 keeps the wasm emitter functions.

### Why 4 GiB is overkill

The patcher's 65536-page (4 GiB) initial memory is a brute-force fix for
the pinned wasm's small 128-page initial memory.  The current compiler
already emits 8192 pages (512 MiB) and has `memory.grow` support
(`helpers_core_heap.ark` → `emit_heap_ensure_grown` called from
`emit_heap_set`, 85+ call sites).  4 GiB is unnecessary; the
performance concern (memory.grow call overhead) can be addressed later.
**Removing the external dependency takes priority over performance
tuning.**

### Root cause analysis (must precede patcher removal)

The patcher masks three distinct root-cause issues.  Each must be fixed
at the source before the patcher can be deleted:

#### 1. Pinned wasm stale memory section (128 pages)

The pinned wasm (`bootstrap/arukellt-selfhost.wasm`) has 128 pages in
its memory section.  No emitter in the codebase history — Rust T1
(256 → 8192), Rust T3 (4), or selfhost (1024 → 8192) — ever emitted 128
pages.  The value likely originates from the original bootstrap artifact
created before ADR-029.  **Fix**: refresh the pinned wasm from current
source so it carries 8192 pages natively.  The patcher's memory
expansion then becomes a no-op.

#### 2. Vec_new missing u32 wraparound detection

`src/compiler/wasm/intrinsic_vec_new_layout.ark` emits a simple bump
allocator (`global.get 0; i32.const N; i32.add; global.set 0`) without
checking for u32 overflow.  The patcher's `patch_vec_new` replaces this
with a wraparound-safe version.  **Fix**: add the overflow check to the
compiler's own `emit_vec_new_write_header` / `emit_vec_new_finish_allocation`
so the patcher's Vec_new replacement is unnecessary.

#### 3. Duplicate export names

The selfhost compiler already has export deduplication logic
(`sections_exports.ark` → `mir_collision_export_name` in
`call_resolve.ark`), which renames colliding exports.  However, the
pinned wasm (built from older source) may predate this logic, or the
deduplication may not cover all cases.  **Fix**: verify the current
compiler never emits duplicate export names; if gaps exist, fix them in
`sections_exports.ark`.  The patcher's `dedupe_export_names` then
becomes unnecessary.

#### 4. MIR prune strips emitter functions (most critical)

The pinned wasm's `lower_to_mir` passes `prune=1`, which strips most
of the wasm emitter when the compiler compiles itself to stage-2
(~345 KiB broken s2).  `_patch_bootstrap_disable_selfhost_mir_prune`
flips this to `prune=0` via binary byte patching.

This is the **most critical** issue: necessary code disappears without
the patch.  **Fix**: refresh the pinned wasm from current source where
`lower_entry_input_to_mir` hardcodes the no-prune path (the modular
pipeline already does this — see `src/compiler/mir/lower/entry.ark`
L17-19).  The binary patch then becomes unnecessary.

### Retirement sequence

1. **Root cause fixes** (compiler-side):
   - Add u32 wraparound check to Vec_new emission
   - Verify export deduplication covers all cases
2. **Refresh pinned wasm** from current source (carries 8192 pages +
   no-prune path + current export dedup natively)
3. **Delete `scripts/bootstrap/wasm-heap-grow-patcher/`** and remove
   from `Cargo.toml` workspace members
4. **Remove patcher calls** from `scripts/selfhost/checks.py`
   (`_ensure_bootstrap_compiler_wasm`, `_ensure_runtime_compiler_wasm`,
   `_dedupe_selfhost_wasm_exports`, `_patch_bootstrap_disable_selfhost_mir_prune`)
5. **Remove patcher build** from `.github/workflows/ci.yml`
6. `python3 scripts/manager.py verify quick` passes without the patcher

## References

- `docs/adr/ADR-007-targets.md` — L204-210: `arukellt_io` abolition and
  WASI P2/P3 import unification
- `docs/adr/ADR-011-wasi-host-layering.md` — `std::host::*` facade policy
- `issues/open/714-wasi-p2-emitter-native-component-output.md`
- `issues/open/675-host-capability-reachability-flags.md`
- `issues/open/668-p2-native-component-polish.md`
- `tools/host-linker/src/host_http.rs`, `host_sockets.rs`
- `src/compiler/wasm/sections_imports.ark`, `import_indices.ark`,
  `emit_target.ark`
- `std/host/http.ark`, `std/host/sockets.ark`
- `std/manifest.toml` (HTTP/sockets availability blocks)
- `scripts/bootstrap/wasm-heap-grow-patcher/src/main.rs` (walrus patcher)
- `scripts/selfhost/checks.py` — `_ensure_bootstrap_compiler_wasm`,
  `_patch_bootstrap_disable_selfhost_mir_prune`,
  `_dedupe_selfhost_wasm_exports`
- `src/compiler/wasm/intrinsics/helpers_core_heap.ark` — `emit_heap_set`,
  `emit_heap_ensure_grown`, `emit_heap_grow_pages`
- `src/compiler/wasm/intrinsic_vec_new_layout.ark` — Vec_new bump allocator
- `src/compiler/wasm/sections_exports.ark` — export deduplication logic
- `src/compiler/wasm/sections_memory.ark` — `initial_memory_pages()`
- `src/compiler/mir/lower/entry.ark` — `lower_entry_input_to_mir` (no-prune path)
