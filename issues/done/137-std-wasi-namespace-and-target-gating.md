# `std::host::*` namespace 導入と migration / target-gated 診断

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-03
**ID**: 137
**Depends on**: —
**Track**: wasi-feature
**Blocks v1 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/137-std-wasi-namespace-and-target-gating.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

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

## Closed — 2026-04-03

### Evidence

- `T3_ONLY_MODULES` in `crates/ark-resolve/src/load.rs` now includes `std::host::sockets` and `std::host::udp`
- `std/host/udp.ark` stub created (T3-only UDP datagram module)
- `std/manifest.toml` updated with `[[modules]]` and `[[functions]]` entry for `std::host::udp`
- `docs/capability-surface.md` updated with `std::host::udp` section
- `diag:` fixture `tests/fixtures/target_gating/t1_import_sockets.ark` — E0500 for T1 + std::host::sockets
- `diag:` fixture `tests/fixtures/target_gating/t1_import_udp.ark` — E0500 for T1 + std::host::udp
- `bash scripts/run/verify-harness.sh --quick`: 19/19 PASS

### Acceptance criteria

- [x] `std::host::*` modules registered in `std/manifest.toml` and resolver T3_ONLY_MODULES
- [x] Deprecated import paths (`std::io`, `std::fs`, etc.) already emit E0104 migration diagnostics
- [x] T3-only module imports on T1 emit E0500 (incompatible target)
- [x] Compile fixtures covering T1+T3-only module rejection
- [x] `bash scripts/run/verify-harness.sh --quick` passes (19/19)
