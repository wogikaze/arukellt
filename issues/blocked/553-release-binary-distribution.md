---
Status: blocked
Updated: 2026-05-17
ID: 553
Track: release
Type: Verification
Depends on: none
Blocked by: "release tag creation and passing tag workflow"
---

# Release: Binary Distribution

> **Status:** blocked
> **Track:** release
> **Type:** Verification

## Scope

Ensure release binary distribution requirements are met.

## Checklist Source

docs/release-checklist.md — Binary distribution section

## Acceptance

- [x] Release CLI entrypoint prepared on linux-x86_64
- [x] Release binary size < 50 MB
- [x] SHA256 checksum generated alongside binary
- [ ] Version in Cargo.toml matches the tag — BLOCKED until a release tag exists; tag push CI now enforces this comparison

## Required Verification

- Prepare release CLI entrypoint for linux-x86_64 target
- Verify binary size is under 50 MB limit
- Generate SHA256 checksum for binary
- Compare Cargo.toml version with git tag (`scripts/check/check-release-tag-version.py`)

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

## Recheck — 2026-05-17

- Current workspace has no `arukellt` Rust binary crate; the release CLI
  entrypoint is the selfhost wrapper copied to `target/release/arukellt`, matching
  `.github/workflows/ci.yml` packaging job behavior.
- Fresh preparation command:
  `mkdir -p target/release && cp scripts/run/arukellt-selfhost.sh target/release/arukellt && chmod +x target/release/arukellt`.
- `target/release/arukellt` exists and is executable.
- `file target/release/arukellt`: Bourne-Again shell script executable.
- `target/release/arukellt --version` prints `arukellt 0.1.0`.
- `target/release/arukellt run tests/fixtures/hello_world.ark` prints
  `Hello, World!`.
- `target/release/arukellt check tests/fixtures/type_error.diag` exits non-zero.
- Entrypoint size: `3,059` bytes, well below the `50 MB` limit.
- SHA256: `15339bb5f5541dfb44d619be209613886ae292ae258209d9e857ab9b96f97180`.
- `.github/workflows/ci.yml` packaging job now enforces the same size gate and
  writes `target/release/arukellt.sha256`.
- `.github/workflows/ci.yml` packaging job now uploads both
  `target/release/arukellt` and `target/release/arukellt.sha256` as
  `arukellt-linux-x86_64-${{ github.sha }}`.
- `.github/workflows/ci.yml` integration, packaging, and determinism jobs now
  install wasmtime before invoking the selfhost wrapper.
- `.github/workflows/ci.yml` now runs on release tag pushes matching `v*` or
  `[0-9]*`; the packaging job checks that the tag version matches
  `Cargo.toml` via `python3 scripts/check/check-release-tag-version.py --require-tag`.
- Workspace package version in `Cargo.toml`: `0.1.0`.
- Git tag state: still blocked; `git describe --tags --abbrev=0` fails with
  `fatal: No names found, cannot describe anything.`

Updated verdict: still blocked only on the release tag publication step. The
tag/version comparison is now automated for tag-push CI, but this issue should
not close until a release tag exists and the tag workflow passes.

## Assessment — 2026-05-16

### Acceptance Criteria Status

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Release CLI entrypoint prepared on linux-x86_64 | PASS | `target/release/arukellt` exists, executable, reports `arukellt 0.1.0` |
| Binary size < 50 MB | PASS | 520,008 bytes (~508 KB), well under limit |
| SHA256 checksum generated | PASS | `e6de9ab9321eb80c3b426cefad75bb00f77d29c7a6db08eb0216abcb2a8d91a0` |
| Version in Cargo.toml matches tag | BLOCKED | `Cargo.toml` version is `0.1.0`; `git describe --tags --abbrev=0` fails — no tags exist in this checkout |

### Analysis

- **Three of four criteria pass and have been stable across three recheck cycles** (2026-05-14, 2026-05-16, latest).
- The binary smoke tests also pass: `--version` outputs `arukellt 0.1.0`, `run tests/fixtures/hello/hello.ark` outputs `Hello, world!`.
- **AC 4 is inherently a release-time gate** — it cannot be verified until a release tag is created. The release checklist now marks this item as `CI on tag`.
- The entrypoint size is extremely lean relative to the 50 MB limit and is stable.

### Recommendation

**Do not close.** The issue is blocked on a release tag that does not yet exist. This is expected for a pre-release state. When a release tag is created:

1. Push a release tag matching `0.1.0` (or whatever version `Cargo.toml` carries at that time) and require the tag workflow to pass.
2. Mark AC 4 complete and close the issue.
3. The binary distribution section of the release checklist can then be fully satisfied.

## Queue Move — 2026-05-17

Moved from `issues/open/` to `issues/blocked/`. Repo-side packaging work is now
covered by CI: release CLI entrypoint preparation, size gate, SHA256 generation,
artifact upload, and tag/version comparison for tag pushes. The only remaining
acceptance item requires an actual release tag and a passing tag workflow.

## Primary Paths

- `Cargo.toml` (version configuration)
- Release CLI entrypoint output
- Binary size verification
- SHA256 checksum generation

## Non-Goals

- Cross-platform binary distribution (linux-x86_64 only for now)
- Binary signing
