---
Status: done
Created: 2026-04-13
Updated: 2026-04-14
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Package registry resolution
**Closed**: 2026-04-14
**ID**: 487
**Depends on**: 039
**Track**: compiler, module-system
**Blocks v1 exit**: no
**Priority**: 50

## Created by audit — 2026-04-13

**Source**: `docs/module-resolution.md` line 160 states "a package registry,
which is not yet implemented." No open issue tracked this gap.

## Summary

Module resolution supports local paths and workspace members, but remote
package registry resolution was not yet implemented. `docs/module-resolution.md`
documented this as a future capability. This issue implemented the resolver
integration (ADR-023, local file-based mock, E0120–E0124 diagnostics).

## Acceptance

- [x] Package registry resolution design documented in ADR
  - Evidence: `docs/adr/ADR-023-package-registry-resolution.md` created 2026-04-14
  - Evidence (acceptance slice): ADR defines registry lookup flow, failure diagnostics (E0120-E0124), and explicit non-goals.
- [x] Registry lookup integrated into resolver import path
  - Evidence: `crates/ark-resolve/src/registry.rs` (new); `crates/ark-resolve/src/load.rs` updated
  - Evidence: `load_program_with_target` builds `RegistryConfig` from nearest `ark.toml`; registry fallback wired into `load_single_import`
- [x] Error diagnostic when registry package is not found
  - Evidence: E0121 emitted for missing package; E0124 for unconfigured registry; E0120 for network-only URL
  - Evidence: `tests/fixtures/modules/registry_not_found/` fixture exercises E0121
- [x] At least 1 fixture tests registry resolution (local mock acceptable)
  - Evidence: `tests/fixtures/modules/registry_not_found/main.ark` + `main.diag` + `ark.toml` + `mock_reg/`
  - Evidence: `bash scripts/run/verify-harness.sh --quick` (19/19 PASS)

## Non-goals

- Hosting a registry service (out of scope)
- Authentication or private registries (follow-up)
- Network/HTTP registry (v1 uses local file-based mock only)

## Primary paths

- `crates/ark-resolve/src/`
- `docs/module-resolution.md`
- `docs/adr/`

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes ✓
- Registry resolution fixture passes ✓

## Close gate

- [x] Design ADR exists (`docs/adr/ADR-023-package-registry-resolution.md`)
- [x] Resolver handles registry import syntax
- [x] docs/module-resolution.md updated to reflect implementation status