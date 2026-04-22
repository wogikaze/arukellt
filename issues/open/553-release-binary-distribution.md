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

## Verification Evidence

- `target/release/arukellt` exists and is executable: `test -x target/release/arukellt` passed.
- Binary size: `529864` bytes, which is below the `50 MB` limit.
- SHA256: `098e3719eaa425494f469b5d8a24a3bfc6900c4e9feffcfb8ab35e4491314df3`.
- `Cargo.toml` package version: `0.1.0`.
- Git tag state: no release tag is available in this checkout (`git describe --tags --abbrev=0` returned `NO_TAG`).

## Verdict

Blocked. The binary distribution evidence is present, but the release tag gate cannot be completed because no git tag is available to compare against the `Cargo.toml` version.

## Primary Paths

- `Cargo.toml` (version configuration)
- Release build output
- Binary size verification
- SHA256 checksum generation

## Non-Goals

- Cross-platform binary distribution (linux-x86_64 only for now)
- Binary signing
