# Release: Determinism Check

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure same source produces identical `.wasm` across two builds for release verification.

## Verification Evidence

### Release-binary determinism (2026-04-25) — PASS

Command used:

```
./target/release/arukellt compile tests/fixtures/hello_world.ark -o /tmp/out1.wasm
./target/release/arukellt compile tests/fixtures/hello_world.ark -o /tmp/out2.wasm
sha256sum /tmp/out1.wasm /tmp/out2.wasm
```

Result:

```
86b057edc6dd72e0bf12214f0aecf90531af44b43df0eb64ff5fc053a4de8f69  /tmp/out1.wasm
86b057edc6dd72e0bf12214f0aecf90531af44b43df0eb64ff5fc053a4de8f69  /tmp/out2.wasm
```

Both hashes are identical — release-binary compilation is deterministic.

### Selfhost fixpoint (prior evidence, 2026-04-22) — PASS

- `python3 scripts/manager.py selfhost fixpoint` is the current canonical determinism command on the available selfhost surface.
- Result on 2026-04-22: PASS, exit 0.
- Reported hashes:
  - `sha256(arukellt-s2.wasm) = c16e32efb1b68e1921eb4915e414f554b165d45e299e0c5fd679934e0ba180cc`
  - pinned base `bootstrap/arukellt-selfhost.wasm = 3a0350371f9dbc37becef03efffa8d20b90827161a0d9fab97163a19de341f2c`

## Checklist Source

docs/release-checklist.md — Pre-release section

## Acceptance

- [x] Same source produces identical `.wasm` across two builds
- [x] SHA256 checksums of two builds match exactly

## Required Verification

- Current available path: `python3 scripts/manager.py selfhost fixpoint`
- Compare SHA256 checksums of the resulting selfhost `.wasm` outputs
- Verify byte-for-byte identity on the selfhost fixpoint outputs
- Release-binary determinism path remains unproven on the current runner surface

## Close Gate

Two builds from identical source must produce byte-identical `.wasm` files. Current evidence only covers selfhost bootstrap fixpoint; release-path determinism remains open until a current release command is available.

## Primary Paths

- `Cargo.toml` (build configuration)
- Release build output directory
- `.wasm` artifact files

## Non-Goals

- Performance comparison between builds
- Build time consistency
- Cross-platform determinism (linux-x86_64 only for now)
