# Capability Surface Reference

> Every host interaction in Arukellt flows through the `std::host::*` namespace.
> This document catalogues every function, its implementation status, target
> compatibility, and the CLI flags that govern capability access.

---

## Host Modules Overview

| Module | Functions | Status | Targets |
|---|---|---|---|
| [`std::host::stdio`](#stdhostsstdio) | 3 | Available | all |
| [`std::host::clock`](#stdhostclock) | 1 | Available | all |
| [`std::host::random`](#stdhostrandom) | 3 | Available | all |
| [`std::host::env`](#stdhostenv) | 5 | Available | all (partial T1) |
| [`std::host::fs`](#stdhostfs) | 3 | Available | all |
| [`std::host::process`](#stdhostprocess) | 2 | Available | all |
| [`std::host::http`](#stdhosthttp) | 2 | Available | all |
| [`std::host::sockets`](#stdhostsockets) | 1 | Available (T3 only) | wasm32-wasi-p2 |

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

HTTP client via TCP-based HTTP/1.1 host functions (Wasmtime linker, T1 and T3).
HTTP**S** is not supported; only plain `http://` URLs work.
Both T1 (wasm32-wasi-p1) and T3 (wasm32-wasi-p2) support this module via the
same `register_http_host_fns` linker registration (issue 446).

| Function | Signature | Status | Targets | Host import |
|---|---|---|---|---|
| `request` | `(String, String, String) -> Result<String, String>` | available | all | `arukellt_host::http_request` |
| `get` | `(String) -> Result<String, String>` | available | all | `arukellt_host::http_get` |

Both functions are wired in the Wasmtime linker via `register_http_host_fns`
(see `crates/arukellt/src/runtime.rs`). They use `std::net::TcpStream` — no
external HTTP client library is required.

#### Error mapping

The following error strings are returned as the `Err` variant:

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

TCP/UDP sockets via WASI Preview 2 `wasi:sockets` interfaces.

| Function | Signature | Status | Targets | WASI import |
|---|---|---|---|---|
| `connect` | `(String, i32) -> Result<i32, String>` | available | wasm32-wasi-p2 | `arukellt_host::sockets_connect` |

Registers a TCP connection to the given hostname and port.  Returns
`Ok(fd)` (fd = 3, placeholder — full fd management is a future extension)
on success, or `Err("connect: <host>:<port>: <reason>")` on failure.
Module stability is **provisional**.  On T1 (wasm32-wasi-p1) a compile-time
diagnostic E0500 (incompatible target) is emitted by the resolver.

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

Programs that reference unimplemented host stubs are rejected at compile
time by a MIR scan. There are currently **no** builtins in the stub list —
`sockets_connect` was promoted to a real T3 host function in issue 447.

(The infrastructure is kept for future host-stub additions.)

`std::host::http` and `std::host::sockets` are both fully wired in the
Wasmtime linker and do not require a host-stub guard.

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
| `http::request` | ✓ | ✓ |
| `http::get` | ✓ | ✓ |
| `sockets::connect` | E0500 | ✓ |

---

## Known Limitations

1. **`env::var` unavailable on T1.** WASI Preview 1 on the T1 backend does
   not import `environ_get`, so `std::host::env::var` is T3-only.

2. **HTTP is available on both T1 and T3.** `std::host::http` is wired via
   `register_http_host_fns` in the Wasmtime linker for both T1 (wasm32-wasi-p1)
   and T3 (wasm32-wasi-p2). `std::host::sockets` is available on T3 only;
   importing it on T1 emits E0500.
   `std::host::http` uses HTTP/1.1 only; HTTPS is not supported.

3. **No `--deny-stdio` flag.** Standard I/O is unconditionally available.

4. **No per-function capability deny.** The deny flags operate at the
   module/category level (clock, random, fs), not per-function.

5. **T3 is in bridge mode.** The wasm32-wasi-p2 backend still uses linear
   memory for WASI I/O. Full GC-native emission is in progress.

6. **Filesystem is deny-by-default but not deny-flagged by default.**
   Without `--dir`, filesystem calls fail at runtime rather than at
   compile time. `--deny-fs` explicitly blocks grants and overrides
   `--dir` flags, but does not add a compile-time scan.
