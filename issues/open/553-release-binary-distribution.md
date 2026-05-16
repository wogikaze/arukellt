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

## Recheck — 2026-05-14

- `target/release/arukellt` exists and is executable.
- `target/release/arukellt --version` prints `arukellt 0.1.0`.
- Binary size: `990400` bytes, below the `50 MB` limit.
- SHA256: `0c675ef4ac0d48c86b11245dfeef73d63161902ec550cd3812669fb3b651f646`.
- Workspace package version in `Cargo.toml`: `0.1.0`.
- Git tag state: still blocked; `git describe --tags --abbrev=0` fails with
  `fatal: No names found, cannot describe anything.`

Updated verdict: still blocked only on the release tag comparison / publication
step. Do not close until a tag exists and matches `0.1.0`.

## Recheck — 2026-05-16

- `target/release/arukellt` exists and is executable (520,008 bytes).
- `target/release/arukellt --version` prints `arukellt 0.1.0`.
- Binary size: 520,008 bytes (~508 KB), well below the 50 MB limit.
- SHA256: `e6de9ab9321eb80c3b426cefad75bb00f77d29c7a6db08eb0216abcb2a8d91a0`.
- Binary smoke tests PASS:
  - `target/release/arukellt --version` -> `arukellt 0.1.0`
  - `target/release/arukellt run tests/fixtures/hello/hello.ark` -> `Hello, world!`
- Workspace package version in `Cargo.toml`: `0.1.0`.
- Git tag state: still blocked; `git describe --tags --abbrev=0` fails with
  `fatal: No names found, cannot describe anything.`

Updated verdict: still blocked only on the release tag comparison / publication
step. All CI-acceptance criteria (binary exists, size < 50 MB, SHA256 generated)
pass. The manual gate "Version in Cargo.toml matches the tag" cannot be verified
until a release tag is created.

## Primary Paths

- `Cargo.toml` (version configuration)
- Release build output
- Binary size verification
- SHA256 checksum generation

## Non-Goals

- Cross-platform binary distribution (linux-x86_64 only for now)
- Binary signing
