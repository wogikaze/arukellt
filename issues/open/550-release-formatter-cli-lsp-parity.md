# Release: Formatter CLI-LSP Parity

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure formatter output matches between CLI and LSP for release verification.

## Checklist Source

docs/release-checklist.md — Extension distribution section

## Acceptance

- [ ] Formatter output matches between CLI and LSP (shared `format_source()`)

## Required Verification

- Format same code using CLI formatter
- Format same code using LSP formatter
- Compare outputs for byte-for-byte identity
- Verify both use shared `format_source()` function

## Close Gate

CLI and LSP formatter outputs must be identical.

## Primary Paths

- CLI formatter implementation
- LSP formatter implementation
- Shared `format_source()` function
- Test fixtures for formatting

## Non-Goals

- Performance comparison
- Feature differences beyond output identity
