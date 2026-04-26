# Pinned-reference selfhost wasm — Provenance

This directory holds the **committed pinned-reference selfhost wasm** that is
the single trusted base for the four canonical selfhost gates (see
[ADR-029](../docs/adr/029-selfhost-native-verification-contract.md)).

The pinned wasm is the source-of-truth for bootstrapping on a fresh clone:
the four gates do **not** require the legacy Rust binary
(`target/debug/arukellt`) and do **not** call `cargo build`.

## Artifact

| Field | Value |
|-------|-------|
| Path | `bootstrap/arukellt-selfhost.wasm` |
| Size | 536 277 bytes (≈ 524 KiB) |
| sha256 | `3a0350371f9dbc37becef03efffa8d20b90827161a0d9fab97163a19de341f2c` |
| Built from commit | `662c3f58` (HEAD of `master` at the time #585 was merged) |
| Build target | `wasm32-wasi-p1` |
| Producer | Selfhost compiler stage 2 (`s2.wasm`), confirmed by Stage-3 fixpoint |

## Reproducibility recipe

The pinned wasm is the deterministic Stage-2 output of the selfhost compiler
when compiled from the recorded source commit:

```bash
# 1. Check out the recorded source commit
git checkout 662c3f58

# 2. Rebuild Stage-1 selfhost wasm using the previous pinned reference
mkdir -p .build/selfhost
wasmtime run --dir=. bootstrap/arukellt-selfhost.wasm -- \
  compile src/compiler/main.ark \
  --target wasm32-wasi-p1 \
  -o .build/selfhost/arukellt-s2.wasm

# 3. Verify byte-for-byte identity with the pinned wasm
sha256sum bootstrap/arukellt-selfhost.wasm .build/selfhost/arukellt-s2.wasm
# ⇒ both sums must be identical (selfhost is at fixpoint).
```

A Stage-3 round confirms the fixpoint:

```bash
wasmtime run --dir=. .build/selfhost/arukellt-s2.wasm -- \
  compile src/compiler/main.ark \
  --target wasm32-wasi-p1 \
  -o .build/selfhost/arukellt-s3.wasm
sha256sum .build/selfhost/arukellt-s2.wasm .build/selfhost/arukellt-s3.wasm
# ⇒ identical sums = fixpoint reached
```

The `selfhost fixpoint` gate (`scripts/selfhost/checks.py::run_fixpoint`)
performs steps 2–3 automatically.

## Refresh policy

The pinned wasm is **explicitly refreshed**, never auto-bumped. Refresh is
required when an intentional behavioural change in the selfhost compiler
(`src/compiler/**`) makes the four gates fail against the previous pinned
reference. Refresh procedure:

1. Locally bootstrap a new Stage-2 wasm from the previous pinned reference and
   the new compiler source (the recipe above, but using the new source HEAD).
2. Verify the Stage-3 fixpoint holds (`s2 == s3`).
3. Run the full fixture-parity gate against the previous pinned reference and
   review every difference. Document each behavioural drift in the refresh
   commit message; if any drift is unintentional, **do not refresh**.
4. Replace `bootstrap/arukellt-selfhost.wasm` with the new Stage-2 binary,
   update this file's *sha256*, *size*, and *Built from commit* rows, and
   commit both changes in one commit titled
   `chore(bootstrap): refresh pinned selfhost wasm to <short-sha>`.

The refresh commit must be signed off by a maintainer and mention every
behavioural drift in its body.

## Why this artifact is committed

The four selfhost gates (`fixpoint`, `fixture-parity`, `diag-parity`,
`cli-parity`) historically required `target/debug/arukellt` (the legacy Rust
compiler) as a trusted base, blocking the Phase 5 retirement work
(#560–#564). The pinned wasm replaces the Rust binary as the trusted base
and is committed so that:

- Fresh clones can bootstrap and verify without any Rust toolchain
- CI never needs to rebuild the Rust crate before running selfhost gates
- The bootstrap base is byte-pinned and reproducible from a git SHA

The artifact size (~524 KiB) is well under the 10 MiB ceiling discussed in
ADR-029. This file is exempted from the repo-wide `*.wasm` ignore in
`.gitignore` via an explicit allow-list entry.
