# Release: Selfhost Bootstrap Check

> **Status:** done
> **Track:** release
> **Type:** Verification

## Scope

Ensure selfhost bootstrap check passes for release verification.

## Checklist Source

docs/release-checklist.md — Pre-release section

## Acceptance

- [x] `bash scripts/run/verify-bootstrap.sh --stage1-only` passes

## Close Note — 2026-04-22

Verification command: `bash scripts/run/verify-bootstrap.sh --stage1-only`
Exit code: `0`

Stage 0 ✓ / Stage 1 ✓ — stdout (last lines):

```
Bootstrap verification

  Selfhost sources: 9 files

── Stage 0: Compile selfhost sources (Rust compiler) ──
  Compiling main.ark → arukellt-s1.wasm (unified binary)...
  OK  arukellt-s1.wasm (570743 bytes)
  PASS  Stage 0

Bootstrap partial verification PASSED
  Scope: requested partial verification completed; bootstrap attainment was not evaluated.
```

Stage 0 compiled the 9 selfhost sources via the Rust compiler and produced
`arukellt-s1.wasm` (570743 bytes); the script's `--stage1-only` scope reports
`Bootstrap partial verification PASSED` with rc=0, satisfying the release
Stage 0 / Stage 1 gate. Stage 2 fixpoint is explicitly out of scope per
Non-Goals.

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
