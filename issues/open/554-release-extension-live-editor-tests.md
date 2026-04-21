# Release: Extension Live Editor Tests

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure extension features work in live editor for release verification.

## Checklist Source

docs/release-checklist.md — Extension distribution section

## Acceptance

- [ ] VSIX installs in VS Code and activates without errors
- [ ] LSP connects and shows "Ready" in language status
- [ ] Diagnostics appear on save for a file with type errors
- [ ] Completion, hover, and go-to-definition work in live editor

## Required Verification

- Install VSIX in VS Code
- Verify extension activation
- Test LSP connection status
- Test diagnostics on type error file
- Test completion, hover, and go-to-definition in live editor

## Close Gate

All extension features must work correctly in live editor environment.

## Primary Paths

- Extension installation process
- LSP connection handling
- Diagnostic display
- Code intelligence features (completion, hover, go-to-definition)

## Non-Goals

- Performance optimization
- Cross-editor testing (VS Code only)
