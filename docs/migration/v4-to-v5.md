# Migration Guide: v4 → v5

> This migration guide covers the transition from Arukellt v4 to v5.

## Overview

v5 introduces the selfhost compiler and bootstrap workflow, but the
repository is still in the dual period. The Rust compiler remains the
default verified compilation path for now, while Stage 2 fixpoint and
parity gates are still unmet. Selfhost is documented and maintained in
parallel with Rust, but it is not yet the primary compilation path; see
[Current State](../current-state.md#self-hosting-bootstrap-status) for the
authoritative verification table and
[Bootstrap Documentation](../compiler/bootstrap.md#dual-period-governance)
for the dual-period policy.

## Breaking Changes

- The selfhost compiler and bootstrap workflow are documented as part of the
  v5 transition, but they remain in the dual period
- The Rust compiler remains the verified compilation path until the
  bootstrap completion criteria are met
- Some CLI flags may differ (see `arukellt help`)
- Component model output uses updated ABI

## Migration Steps

1. Update toolchain: `mise install`
2. Verify the Rust path still builds: `arukellt build`
3. Check for deprecated APIs while the Rust and selfhost compilers remain
   in parallel: `arukellt check`

## Selfhost bootstrap (`scripts/run/verify-bootstrap.sh`)

Bootstrap verification is a three-stage gate defined and labeled by
`scripts/run/verify-bootstrap.sh`:

| Stage | What runs | Artifact / outcome |
|-------|-----------|-------------------|
| **0** | Rust `arukellt` compiles `src/compiler/main.ark` with `--target wasm32-wasi-p1` | `.bootstrap-build/arukellt-s1.wasm` |
| **1** | `wasmtime run` executes `arukellt-s1.wasm` with the repo root mounted; it compiles the same `main.ark` to the same target | `.bootstrap-build/arukellt-s2.wasm` |
| **2** | `sha256sum` on both wasm files | Success when hashes match (fixpoint) |

**Compiler binary** (Stage 0): the script uses `ARUKELLT_BIN` if set, otherwise
`target/debug/arukellt`, otherwise `target/release/arukellt` (the file must
exist and be executable — see the script’s pre-flight checks).

**Prerequisite** (Stage 1): `wasmtime` must be on `PATH`.

**Useful options** (from the script’s `--help` text):

- Default invocation runs Stage **0 → 1 → 2** (full bootstrap gate).
- `--stage1-only` runs **Stage 0 only**, then exits; it does not evaluate
  bootstrap attainment.
- `--stage N` runs a single stage (`0`, `1`, or `2`).
- `--fixture-parity` after Stage 0 runs `scripts/check/check-selfhost-parity.sh --fixture` when that helper exists.
- `--check` prints machine-readable stage status (`stage0-compile`,
  `stage1-self-compile`, `stage2-fixpoint`, `attainment`) for the **full**
  gate only; it cannot be combined with `--stage`, `--stage1-only`, or
  `--fixture-parity`.

Artifacts are written under `.bootstrap-build/` and removed when the script
exits (the script installs an `EXIT` trap that deletes that directory).

For deeper debugging and governance, see
[Bootstrap documentation](../compiler/bootstrap.md).

**Last audited:** 2026-04-16 against `scripts/run/verify-bootstrap.sh` in this
repository.

## See Also

- [Bootstrap Documentation](../compiler/bootstrap.md)
- [Current State](../current-state.md)
