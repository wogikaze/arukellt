# Capability Surface Reference

> Every host interaction in Arukellt flows through the `std::host::*` namespace.
> This document catalogues every function, its implementation status, target
> compatibility, and the CLI flags that govern capability access.

---

## Host Modules Overview

| Module | Functions | Status | Targets |
|---|---|---|---|
| [`std::host::stdio`](#stdhoststdio) | 3 | Available | all |
| [`std::host::clock`](#stdhostclock) | 1 | Available | all |
| [`std::host::random`](#stdhostrandom) | 3 | Available | all |
| [`std::host::env`](#stdhostenv) | 5 | Available | all (partial T1) |
| [`std::host::fs`](#stdhostfs) | 3 | Available | all |
| [`std::host::process`](#stdhostprocess) | 2 | Available | all |
| [`std::host::http`](#stdhosthttp) | 2 | Not user-reachable | — |
| [`std::host::sockets`](#stdhostsockets) | 1 | Not user-reachable | — |
| [`std::host::udp`](#stdhostudp) | 1 | Not user-reachable | — |

---

## Function Reference

### `std::host::stdio`

Standard output and error streams.

| Function | Signature | Status | Targets | WASI import |
|---|---|---|---|---|
| `print` | `(String) -> ()` | available | all | `fd_write` (fd 1) |
| `println` | `(String) -> ()` | available | all | `fd_write` (fd 1) |
| `eprintln` | `(String) -> ()` | available | all | `fd_write` (fd 2) |

Stdio is always granted; there is no `--deny-stdio` flag.

---

### `std::host::clock`

Monotonic clock access via WASI `clock_time_get`.

| Function | Signature | Status | Targets | WASI import |
|---|---|---|---|---|
| `monotonic_now` | `() -> i64` | available | all | `clock_time_get` (clock\_id 0) |

Returns nanoseconds as `i64`. Denied at compile time by `--deny-clock`.

---

### `std::host::random`

Cryptographic-quality entropy via WASI `random_get`.

| Function | Signature | Status | Targets | WASI import |
|---|---|---|---|---|
| `random_i32` | `() -> i32` | available | all | `random_get` |
| `random_i32_range` | `(i32, i32) -> i32` | available | all | `random_get` |
| `random_bool` | `() -> bool` | available | all | `random_get` |

All three share the `__intrinsic_random_i32` intrinsic. Denied at compile
time by `--deny-random`.

---

### `std::host::env`

Program arguments and environment variables.

| Function | Signature | Status | Targets | WASI import |
|---|---|---|---|---|
| `args` | `() -> Vec<String>` | available | all | `args_sizes_get`, `args_get` |
| `arg_count` | `() -> i32` | available | all | `args_sizes_get` |
| `arg_at` | `(i32) -> Option<String>` | available | all | `args_sizes_get`, `args_get` |
| `var` | `(String) -> Option<String>` | available | T3 only | `environ_get` |
| `has_flag` | `(String) -> bool` | available | all | *(pure Ark — no host call)* |

`var` is **not available on the T1 backend** (wasm32-wasi-p1) because WASI
Preview 1 does not expose `environ_get` in the T1 import set. On T3
(wasm32-wasi-p2) it works via `environ_get`.

---

### `std::host::fs`

Filesystem read/write via WASI preopened directories.

| Function | Signature | Status | Targets | WASI imports |
|---|---|---|---|---|
| `read_to_string` | `(String) -> Result<String, String>` | available | all | `path_open`, `fd_read`, `fd_close` |
| `write_string` | `(String, String) -> Result<(), String>` | available | all | `path_open`, `fd_write`, `fd_close` |
| `write_bytes` | `(String, Vec<i32>) -> Result<(), String>` | available | all | `path_open`, `fd_write`, `fd_close` |
| `is_readable_file` | `(String) -> bool` | available (read probe) | all | same as `read_to_string` |
| `exists` | `(String) -> bool` | available (deprecated probe) | all | same as `read_to_string` |
| `is_file` | `(String) -> bool` | available (probe, not type check) | all | same as `read_to_string` |
| `is_dir` | `(String) -> bool` | honest stub (always `false`) | all | — |
| `read_dir` | `(String) -> Result<Vec<String>, FsError>` | honest rejection | all | — |
| `metadata` | `(String) -> Result<FsMetadata, FsError>` | honest rejection | all | — |

`is_readable_file` / `exists` / `is_file` are **read probes**, not path-existence or
file-type queries. `is_dir` always returns `false` until directory metadata
intrinsics exist. `read_dir` and `metadata` always return `Err` documenting the
capability gap — see `std/host/fs.ark` and issue #637.

Filesystem access is **deny-by-default**. A directory must be explicitly
granted with `--dir` before any of these functions can succeed. See
[CLI Capability Flags](#cli-capability-flags) below.

---

### `std::host::process`

Process lifecycle primitives.

| Function | Signature | Status | Targets | WASI import |
|---|---|---|---|---|
| `exit` | `(i32) -> ()` | available | all | `proc_exit` (`wasi_snapshot_preview1`) |
| `abort` | `() -> ()` | available | all | `proc_exit(134)` (`wasi_snapshot_preview1`) |

`exit(code)` calls the WASI `proc_exit` host function directly, terminating the
process immediately with the given exit code. `abort()` calls `proc_exit(134)`,
following the SIGABRT convention. Both are **noreturn**: the emitter emits
`unreachable` after every call site. Neither function is subject to capability
gating (see Issue 448 for future `--deny-process` support).

---

### `std::host::http`

HTTP client helpers are **T3-only** (`wasm32-wasi-p2`). T1 import emits E0500 (#655).
P2 emitter imports `wasi:http/outgoing-handler@0.2.0` and lowers via `arukellt_host` bridge.
**HTTPS not supported.**

| Function | Signature | Status | Targets | Backing |
|---|---|---|---|---|
| `request` | `(String, String, String) -> Result<String, String>` | available (bridge) | T3 | `arukellt_host` + outgoing-handler import |
| `get` | `(String) -> Result<String, String>` | available (bridge) | T3 | same |
| `request_with_headers` | returns `HttpResponse` | provisional | T3 | whole-body; headers not forwarded yet |

#### Error mapping (when implemented)

| Situation | `Err(String)` value |
|---|---|
| DNS resolution failure | `"dns: <hostname>: not found"` |
| Connection refused | `"connection refused: <url>"` |
| Connection / read timeout | `"timeout: <url>"` |
| HTTP 4xx or 5xx status | `"http <status>: <url>"` |
| HTTPS URL (unsupported) | `"https is not supported (TCP HTTP/1.1 only)"` |
| Any other I/O failure | `"error: <message>"` |

Module stability is **provisional** (HTTPS not supported). There is no `--deny-http` flag.

---

### `std::host::sockets`

**Not user-reachable** on the selfhost path.
[#447](../issues/done/447-std-host-sockets-implementation.md) /
[#139](../issues/open/139-std-wasi-sockets-p2.md).

| Function | Signature | Status | Targets | Backing |
|---|---|---|---|---|
| `connect` | `(String, i32) -> Result<i32, String>` | not user-reachable | — | #447 / #139 |

---

### `std::host::udp`

**Not user-reachable** on the selfhost path.
[#139](../issues/open/139-std-wasi-sockets-p2.md).

| Function | Signature | Status | Targets | Backing |
|---|---|---|---|---|
| `send` | `(String, i32, String) -> Result<i32, String>` | not user-reachable | — | #139 |

---

## CLI Capability Flags

### Deny flags

| Flag | Scope | Enforcement | Effect |
|---|---|---|---|
| `--deny-fs` | Filesystem | Runtime (WASI) | Blocks all directory grants; overrides any `--dir` flags. No preopened directories are mounted. |
| `--deny-clock` | Clock | Compile-time (MIR scan) | Hard error if the program references any clock intrinsic. |
| `--deny-random` | Random | Compile-time (MIR scan) | Hard error if the program references any random intrinsic. |

### Directory grant

| Flag | Scope | Enforcement | Effect |
|---|---|---|---|
| `--dir PATH` | Filesystem | Runtime (WASI preopened dir) | Grants read-write access to `PATH`. |
| `--dir PATH:ro` | Filesystem | Runtime (WASI preopened dir) | Grants read-only access to `PATH`. |
| `--dir PATH:rw` | Filesystem | Runtime (WASI preopened dir) | Grants read-write access to `PATH` (explicit). |

### Default policy

| Capability | Default | Notes |
|---|---|---|
| Standard I/O | **Allow** | Cannot be denied. |
| Filesystem | **Deny** | No access unless `--dir` grants a directory. |
| Clock / Time | **Allow** | Available unless `--deny-clock` is passed. |
| Random | **Allow** | Available unless `--deny-random` is passed. |

---

## Host Stub Enforcement

No builtins in the stub list. `std::host::http`, `std::host::sockets`, and
`std::host::udp` are **not user-reachable** on the selfhost path — see
[#633](../issues/done/633-host-capability-surface-honesty-vs-selfhost-runtime.md),
[#446](../issues/done/446-std-host-http-implementation.md),
[#447](../issues/done/447-std-host-sockets-implementation.md),
[#077](../issues/open/077-wasi-p2-http.md),
[#139](../issues/open/139-std-wasi-sockets-p2.md).

---

## Capability Scan Internals

The compiler enforces `--deny-clock` and `--deny-random` by scanning the
compiled MIR (`mir_uses_capability()`). The scan walks all functions,
blocks, and nested expressions to detect calls to blocked builtins:

| Deny flag | Blocked builtins |
|---|---|
| `--deny-clock` | `clock_now`, `clock_now_ms`, `monotonic_now`, `__intrinsic_clock_now`, `__intrinsic_clock_now_ms` |
| `--deny-random` | `random_i32`, `random_f64`, `__intrinsic_random_i32`, `__intrinsic_random_f64` |

Detection is transitive — if function A calls function B which calls a
blocked intrinsic, the program is still rejected.

---

## Target Compatibility Matrix

| Function | T1 (wasm32-wasi-p1) | T3 (wasm32-wasi-p2) |
|---|---|---|
| `stdio::print` | ✓ | ✓ |
| `stdio::println` | ✓ | ✓ |
| `stdio::eprintln` | ✓ | ✓ |
| `clock::monotonic_now` | ✓ | ✓ |
| `random::random_i32` | ✓ | ✓ |
| `random::random_i32_range` | ✓ | ✓ |
| `random::random_bool` | ✓ | ✓ |
| `env::args` | ✓ | ✓ |
| `env::arg_count` | ✓ | ✓ |
| `env::arg_at` | ✓ | ✓ |
| `env::var` | ✗ | ✓ |
| `env::has_flag` | ✓ | ✓ |
| `fs::read_to_string` | ✓ | ✓ |
| `fs::write_string` | ✓ | ✓ |
| `fs::write_bytes` | ✓ | ✓ |
| `process::exit` | ✓ | ✓ |
| `process::abort` | ✓ | ✓ |
| `http::request` | — | — |
| `http::get` | — | — |
| `sockets::connect` | E0500 | — |

---

## Known Limitations

1. **`env::var` unavailable on T1.** WASI Preview 1 on the T1 backend does
   not import `environ_get`, so `std::host::env::var` is T3-only.

2. **HTTP/sockets/UDP not user-reachable (#633).** See #446/#447/#077/#139.
   HTTPS not supported for HTTP.

3. **No `--deny-stdio` flag.** Standard I/O is unconditionally available.

4. **No per-function capability deny.** The deny flags operate at the
   module/category level (clock, random, fs), not per-function.

5. **T3 is in bridge mode.** The wasm32-wasi-p2 backend still uses linear
   memory for WASI I/O. Full GC-native emission is in progress.

6. **Filesystem is deny-by-default but not deny-flagged by default.**
   Without `--dir`, filesystem calls fail at runtime rather than at
   compile time. `--deny-fs` explicitly blocks grants and overrides
   `--dir` flags, but does not add a compile-time scan.

---

## Runtime verification

T1/T3 execution checks for the ADR-011 `std::host::*` rollout are registered in
`tests/fixtures/manifest.txt` and enforced by close gates. Use
[`migration/t1-to-t3.md`](migration/t1-to-t3.md) for target selection (`wasm32-wasi-p1` vs
`wasm32-wasi-p2`).

| Capability group | Fixtures | Close gate | Issue |
|---|---|---|---|
| Shared (`stdio`, `fs`, `env`, `process`, `clock`, `random`) | `tests/fixtures/stdlib_host/wasi_{stdio,fs,args,process,clock,random}.ark` (`run:` + `t3-run:`) | `scripts/check/gate-138-shared-capabilities-t1-t3.py` | [#138](../issues/done/138-std-wasi-shared-capabilities-t1-t3.md) |
| HTTP outgoing client | `tests/fixtures/host/http/get_err_dns.ark`, `tests/fixtures/wasi_http_p2.ark` | `scripts/check/gate-655-http-outgoing.py` | [#655](../issues/done/655-wasi-p2-http-outgoing-client.md) (parent [#077](../issues/open/077-wasi-p2-http.md)) |
| Sockets connect/read/write | `tests/fixtures/host/sockets/connect_read_write.ark` | `scripts/check/gate-657-sockets-connect-read-write.py` | [#657](../issues/done/657-std-wasi-sockets-connect-read.md) (parent [#139](../issues/open/139-std-wasi-sockets-p2.md)) |
| Namespace + T1 reject for P2-only modules | `tests/fixtures/target_gating/t1_import_{sockets,udp,http}.ark` | resolver target gate (#137) | [#137](../issues/done/137-std-wasi-namespace-and-target-gating.md) |

Remaining P2 slices (issue #656 HTTP server, issue #658 sockets listen/accept) stay on the open
queue; they do not block the ADR-011 namespace rollout tracked by issue #136.
