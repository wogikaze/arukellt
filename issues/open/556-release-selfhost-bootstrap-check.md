# Release: Selfhost Bootstrap Check

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure selfhost bootstrap check passes for release verification.

## Checklist Source

docs/release-checklist.md — Pre-release section

## Acceptance

- [ ] `bash scripts/run/verify-bootstrap.sh --stage1-only` passes

## Required Verification

- Run selfhost bootstrap verification with stage1-only flag
- Verify Stage 0 and Stage 1 compilation succeed
- Ensure bootstrap reaches fixpoint

## Close Gate

Selfhost bootstrap Stage 0 and Stage 1 must pass.

## Primary Paths

- `scripts/run/verify-bootstrap.sh` (bootstrap verification script)
- Selfhost compiler implementation
- Bootstrap test fixtures

## Non-Goals

- Stage 2 fixpoint (not required for release)
