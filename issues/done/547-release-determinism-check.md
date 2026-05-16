# Release: Determinism Check

> **Status:** done
> **Track:** release
> **Type:** Verification

## Scope

Ensure same source produces identical `.wasm` across two builds for release
verification.

## Verification Evidence

### Release-binary determinism (2026-05-14) — PASS

Command used:

```bash
./target/release/arukellt compile tests/fixtures/hello_world.ark -o /tmp/arukellt_det1.wasm
./target/release/arukellt compile tests/fixtures/hello_world.ark -o /tmp/arukellt_det2.wasm
sha256sum /tmp/arukellt_det1.wasm /tmp/arukellt_det2.wasm
```

Result:

```text
86b057edc6dd72e0bf12214f0aecf90531af44b43df0eb64ff5fc053a4de8f69  /tmp/arukellt_det1.wasm
86b057edc6dd72e0bf12214f0aecf90531af44b43df0eb64ff5fc053a4de8f69  /tmp/arukellt_det2.wasm
```

Both hashes are identical, so the release-binary compilation path is
deterministic for the release smoke fixture.

## Checklist Source

docs/release-checklist.md — Pre-release section

## Acceptance

- [x] Same source produces identical `.wasm` across two builds
- [x] SHA256 checksums of two builds match exactly

## Required Verification

- Build release binary: `cargo build --release -p arukellt`
- Compile the same source twice with the release binary
- Compare SHA256 checksums of both `.wasm` outputs

## Close Gate

Two builds from identical source produce byte-identical `.wasm` files.

## Primary Paths

- `Cargo.toml` (build configuration)
- Release build output directory
- `.wasm` artifact files

## Non-Goals

- Performance comparison between builds
- Build time consistency
- Cross-platform determinism (linux-x86_64 only for now)
