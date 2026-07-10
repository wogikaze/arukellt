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
