# Migration Guide: v4 → v5

> This migration guide covers the transition from Arukellt v4 to v5.
> For current behavior, see [`../current-state.md`](../current-state.md).

## Overview

v5 completes the self-hosting transition. The selfhost compiler is now the **only** execution path. The dual period has ended. The selfhost fixpoint has been reached and all parity gates pass with 0 failures.

- Fixpoint: `sha256(s2) == sha256(s3)` – PASS
- Fixture parity: 321 PASS / 0 FAIL / 41 SKIP
- CLI parity: 6 PASS / 0 FAIL
- Diagnostic parity: 12 PASS / 0 FAIL

If you are upgrading from v4, read this guide before building.

## Breaking Changes

### 1. Selfhost-only execution path

The `arukellt` CLI now runs exclusively through the selfhost wasm under `wasmtime`. There is no longer a Rust binary providing compilation.

**`ARUKELLT_USE_RUST=1` is retired**: exits non-zero with a migration notice. Remove any scripts that set this variable.

**`cargo run -p arukellt`** is no longer used anywhere for compilation.

New selfhost wasm resolution order:
1. `$ARUKELLT_SELFHOST_WASM` (explicit override)
2. `.build/selfhost/arukellt-s2.wasm` (fresh build)
3. `.bootstrap-build/arukellt-s2.wasm` (bootstrap intermediate)
4. `bootstrap/arukellt-selfhost.wasm` (committed pinned reference)

User-facing commands are unchanged (`arukellt compile`, `run`, `check`, etc.).

### 2. Rust crates removed

The following Rust crates have been deleted from `crates/`:

| Crate | Issue | Replacement |
|-------|-------|-------------|
| `crates/ark-driver` | #560 | `src/compiler/driver.ark` |
| `crates/ark-mir` | #561 | `src/compiler/passes/` |
| Rust Wasm emitter crate | #562 | `src/compiler/emitter.ark` |
| `crates/ark-llvm` | #586 | future selfhost-native T4 |
| `crates/ark-lsp` | #572 | `src/ide/lsp.ark` |

If your build scripts reference any of these crates, remove those references.

### 3. Shell scripts replaced

| Removed script | Replacement |
|----------------|-------------|
| `scripts/run/verify-harness.sh` | `python scripts/manager.py verify` |
| `scripts/check/check-selfhost-*.sh` | `python scripts/manager.py selfhost` |
| `scripts/check/perf-gate.sh` | `python scripts/manager.py perf` |
| `scripts/run/run-benchmarks.sh` | `python scripts/manager.py bench` |
| `scripts/gate/*.sh` | `python scripts/manager.py gate` |

Update CI configs and local scripts to use `python scripts/manager.py`.

### 4. `std::host::fs` error type changed

`std::host::fs` functions now return typed `FsError` instead of `String`:

| Function | Old return | New return |
|----------|-----------|-----------|
| `read_to_string(path)` | `Result<String, String>` | `Result<String, FsError>` |
| `write_string(path, contents)` | `Result<(), String>` | `Result<(), FsError>` |
| `write_bytes(path, bytes)` | `Result<(), String>` | `Result<(), FsError>` |

FsError variants: NotFound(String), PermissionDenied(String), Utf8Error, IoError(String).
Use `fs::fs_error_message(err)` for a plain string message.

Update error handlers from `Err(msg: String)` to `Err(e: FsError)`.

### 5. `std::json::parse` error type changed

`parse(s)` now returns `Result<JsonValue, JsonParseError>` (was `Result<JsonValue, String>`).

JsonParseError variants: EmptyInput, InvalidLiteral, TrailingCharacters, UnexpectedCharacter.

### 6. Prelude scope narrowed

Some stdlib functions now require explicit `use` imports. Run `arukellt check` to identify affected code.

## Migration Steps

1. Remove `ARUKELLT_USE_RUST=1` from scripts and environment
2. Ensure `wasmtime` is installed: `mise install`
3. Update `fs::` error handlers (Breaking Change 4)
4. Update `json::parse` error handlers (Breaking Change 5)
5. Add explicit `use` imports where needed (`arukellt check`)
6. Replace shell scripts with `python scripts/manager.py` subcommands
7. Verify: `python scripts/manager.py verify quick`

## Bootstrap Status (v5)

The selfhost fixpoint is verified by the pinned reference at `bootstrap/arukellt-selfhost.wasm` (ADR-029):

- sha256(s2) == sha256(s3) fixpoint: PASS
- Fixture parity: 321 PASS / 0 FAIL
- Diagnostic parity: 12 PASS / 0 FAIL
- CLI parity: 6 PASS / 0 FAIL

See [Bootstrap Documentation](../compiler/bootstrap.md) and [Current State](../current-state.md) for details.

## See Also

- [Bootstrap Documentation](../compiler/bootstrap.md)
- [Current State](../current-state.md)
- [CHANGELOG](../../CHANGELOG.md)
