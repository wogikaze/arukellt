# Package registry resolution

**Status**: open
**Created**: 2026-04-13
**Updated**: 2026-04-13
**ID**: 487
**Depends on**: 039
**Track**: compiler, module-system
**Blocks v1 exit**: no
**Priority**: 50

## Created by audit — 2026-04-13

**Source**: `docs/module-resolution.md` line 160 states "a package registry, which is not yet implemented." No open issue tracked this gap.

## Summary

Module resolution supports local paths and workspace members, but remote package registry resolution is not yet implemented. `docs/module-resolution.md` documents this as a future capability.

## Acceptance

- [ ] Package registry resolution design documented in ADR
- [ ] Registry lookup integrated into resolver import path
- [ ] Error diagnostic when registry package is not found
- [ ] At least 1 fixture tests registry resolution (local mock acceptable)

## Non-goals

- Hosting a registry service (out of scope)
- Authentication or private registries (follow-up)

## Primary paths

- `crates/ark-resolve/src/`
- `docs/module-resolution.md`
- `docs/adr/`

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes
- Registry resolution fixture passes

## Close gate

- Design ADR exists
- Resolver handles registry import syntax
- docs/module-resolution.md updated to reflect implementation status
