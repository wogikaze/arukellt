# Release: Failure Recovery Tests

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure failure recovery mechanisms work correctly for release verification.

## Checklist Source

docs/release-checklist.md — Failure recovery section

## Acceptance

- [ ] After killing the LSP process, the extension shows "Error" status
- [ ] `Arukellt: Restart Language Server` command restarts successfully
- [ ] Invalid `arukellt.path` setting shows a clear error message
- [ ] Missing `ark.toml` gracefully falls back to single-file mode

## Required Verification

- Test LSP process crash recovery
- Test manual restart command
- Test invalid configuration error handling
- Test missing project file fallback behavior

## Close Gate

All failure recovery scenarios must handle errors gracefully with clear user feedback.

## Primary Paths

- Extension error handling code
- LSP process management
- Configuration validation
- Project file detection and fallback

## Non-Goals

- Performance optimization
- Additional error scenarios beyond checklist
