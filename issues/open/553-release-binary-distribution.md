# Release: Binary Distribution

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure release binary distribution requirements are met.

## Checklist Source

docs/release-checklist.md — Binary distribution section

## Acceptance

- [ ] Release binary built for linux-x86_64
- [ ] Release binary size < 50 MB
- [ ] SHA256 checksum generated alongside binary
- [ ] Version in Cargo.toml matches the tag

## Required Verification

- Build release binary for linux-x86_64 target
- Verify binary size is under 50 MB limit
- Generate SHA256 checksum for binary
- Compare Cargo.toml version with git tag

## Close Gate

All binary distribution requirements must be satisfied.

## Primary Paths

- `Cargo.toml` (version configuration)
- Release build output
- Binary size verification
- SHA256 checksum generation

## Non-Goals

- Cross-platform binary distribution (linux-x86_64 only for now)
- Binary signing
