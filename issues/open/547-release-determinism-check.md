# Release: Determinism Check

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure same source produces identical `.wasm` across two builds for release verification.

## Checklist Source

docs/release-checklist.md — Pre-release section

## Acceptance

- [ ] Same source produces identical `.wasm` across two builds
- [ ] SHA256 checksums of two builds match exactly

## Required Verification

- Build release binary twice from same source
- Compare SHA256 checksums of resulting `.wasm` files
- Verify byte-for-byte identity

## Close Gate

Two builds from identical source must produce byte-identical `.wasm` files.

## Primary Paths

- `Cargo.toml` (build configuration)
- Release build output directory
- `.wasm` artifact files

## Non-Goals

- Performance comparison between builds
- Build time consistency
- Cross-platform determinism (linux-x86_64 only for now)
