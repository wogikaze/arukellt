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

## See Also

- [Bootstrap Documentation](../compiler/bootstrap.md)
- [Current State](../current-state.md)
