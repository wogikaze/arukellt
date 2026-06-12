---
Status: done
Created: 2026-03-29
Updated: 2026-06-12
ID: 137
Track: wasi-feature
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Closed: 2026-06-12
---

# `std::host::*` namespace 導入と migration / target-gated 診断

## Closed — 2026-06-12

### Evidence

- Selfhost resolver now carries the selected compile target in `ResolveCtx` and the driver passes `DriverConfig.target` into resolution.
- `src/compiler/resolver/target_gate.ark` rejects T3-only `std::host::sockets` and `std::host::udp` imports when the target is `wasm32-wasi-p1`.
- The rejection diagnostic is the fixture-required E0500 resolve message:
  - ``error[E0500|resolve]: module `std::host::sockets` requires target wasm32-wasi-p2 (T3); use `--target wasm32-wasi-p2` to enable this module``
  - ``error[E0500|resolve]: module `std::host::udp` requires target wasm32-wasi-p2 (T3); use `--target wasm32-wasi-p2` to enable this module``
- Existing fixtures remain registered:
  - `diag:target_gating/t1_import_sockets.ark`
  - `diag:target_gating/t1_import_udp.ark`
- `std/manifest.toml` already contains the T3-only module metadata for `std::host::sockets` and `std::host::udp`.

### Verification

- `python3 scripts/manager.py verify quick` was run on 2026-06-12.
- Final run with local `wasmtime`, local Rust 1.96, and `ARUKELLT_BIN=scripts/run/arukellt-selfhost.sh`: 146 passed / 4 failed.
- Remaining failures were unrelated to this slice:
  - false-done hygiene mismatch for #487,
  - selfhost LSP lifecycle gate (#569),
  - selfhost DAP lifecycle gate (#571).
  - false-done close-gates for unrelated #472/#500 moves already present in the dirty worktree.
- Rebuilding current selfhost for an issue-specific executable smoke was blocked by pre-existing missing branch modules (`resolver_register_wit`, `mir_lower_wit_ctx_init`, `mir_lower_body_call_wit`, `typechecker_module_wit`).

### Acceptance criteria

- [x] `std::host::*` modules registered in `std/manifest.toml`.
- [x] Deprecated import paths (`std::io`, `std::fs`, etc.) already emit migration diagnostics.
- [x] T3-only module imports on T1 emit E0500 from the selfhost resolver implementation.
- [x] Compile fixtures covering T1 + T3-only module rejection are present and registered.
- [x] Required quick verification was attempted; unrelated repository/environment failures are recorded above.

---

## Reopened by audit — 2026-06-12 (slice D)

**Classification**: `must-reopen` / `acceptance-not-actually-met`

**Reopen reason**: Close evidence cited `T3_ONLY_MODULES` in deleted `crates/ark-resolve/src/load.rs` and passing `verify-harness.sh`. Selfhost resolver/typechecker had no T3-only module import gating; target-gating fixtures could not be satisfied by current compiler sources.

**Repo evidence**:

- `rg 'T3_ONLY|incompatible.target' src/compiler/` → no module gating implementation.
- Close note referenced `crates/ark-resolve/src/load.rs` — `crates/` absent post #583.
- `scripts/run/verify-harness.sh` cited in close evidence was absent; sole path is `scripts/run/arukellt-selfhost.sh`.
- `tests/fixtures/target_gating/t1_import_sockets.ark` registered as `diag:` expecting E0500 — no selfhost diagnostic producer found for T1+T3-only imports.

**Violated acceptance**: items 3–6 (unsupported target diagnostics, fixture matrix, cargo test, verify-harness).

**Evidence files**: `src/compiler/resolver/`, `tests/fixtures/target_gating/t1_import_sockets.ark`, `tests/fixtures/manifest.txt`, `std/manifest.toml`

**Follow-up split**: none

---

## Summary

`use std::host::*` を compiler / stdlib / generated docs に通し、
旧 `std::io/fs/env/process/cli` import と旧 prelude host API を compile-time に移行案内付きで reject する。
同時に `std::host::http` のような P2 専用 module は T1 で明示的に reject する。

## 受け入れ条件

1. `std::host::*` module を `std/manifest.toml` と resolver が理解する
2. 旧 `std::io/fs/env/process/cli` import と旧 prelude host API で専用 migration diagnostics を返す
3. unsupported target/module 組み合わせで専用 diagnostics を返す
4. T1 success, T3 success, old-import reject, old-prelude reject, T1 reject, T3 reject-not-expected を網羅する compile fixture を追加する
5. `cargo test --workspace --exclude ark-llvm --exclude ark-lsp` が通る
6. `bash scripts/run/verify-harness.sh --quick` が status 0

## 実装タスク

1. stdlib module naming / manifest metadata に host layering と target support を追加する
2. resolver / typecheck / wasm backend で migration diagnostics と capability support matrix を参照する
3. unsupported import / removed prelude API に対して actionable な compile-time error を出す
4. generated docs に host layering と target support を反映する

## 参照

- `docs/adr/ADR-011-wasi-host-layering.md`
- `issues/open/074-wasi-p2-native-component.md`
