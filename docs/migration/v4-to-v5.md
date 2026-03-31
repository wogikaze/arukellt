# Migration Guide: v4 → v5

> This migration guide covers the transition from Arukellt v4 to v5.

## Overview

v5 introduces the selfhost compiler as the primary compilation path.
During the dual period, both Rust and selfhost compilers are available.

## Breaking Changes

- Selfhost compiler becomes the canonical binary
- Some CLI flags may differ (see `arukellt help`)
- Component model output uses updated ABI

## Migration Steps

1. Update toolchain: `mise install`
2. Verify builds: `arukellt build`
3. Check for deprecated APIs: `arukellt check`

## See Also

- [Bootstrap Documentation](../compiler/bootstrap.md)
- [Current State](../current-state.md)
