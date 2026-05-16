# Release: Failure Recovery Tests

> **Status:** done
> **Track:** release
> **Type:** Verification
> **Updated:** 2026-05-14

## Scope

Ensure failure recovery mechanisms work correctly for release verification.

## Checklist Source

docs/release-checklist.md — Failure recovery section

## Acceptance

- [x] After killing the LSP process, the extension shows "Error" status
- [x] `Arukellt: Restart Language Server` command restarts successfully
- [x] Invalid `arukellt.server.path` setting shows a clear error message
- [x] Missing `ark.toml` gracefully falls back to single-file mode

## Required Verification

- Test LSP process crash recovery: covered by `npm test` in `extensions/arukellt-all-in-one/src/test/extension.test.js`.
- Test manual restart command: covered by the stub LSP restart E2E tests.
- Test invalid configuration error handling: covered by the missing/invalid binary status test.
- Test missing project file fallback behavior: covered by the single-file mode E2E test with no `ark.toml`.

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
