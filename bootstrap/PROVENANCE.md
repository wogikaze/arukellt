# Pinned-reference selfhost wasm — Provenance

This directory holds the **committed pinned-reference selfhost wasm** that is
the single trusted base for the four canonical selfhost gates (see
[ADR-029](../docs/adr/ADR-029-selfhost-native-verification-contract.md)).

The pinned wasm is the source-of-truth for bootstrapping on a fresh clone.
It is a direct WASI Preview 1 core module executed by Wasmtime; no Rust binary,
Cargo workspace, host-linker, or repository-specific runtime is involved.

## Artifact

| Field | Value |
|-------|-------|
| Path | `bootstrap/arukellt-selfhost.wasm` |
| Size | 4 870 797 bytes (≈ 4.65 MiB) |
| sha256 | `df6b20ab640435c3686cb05249028654a3c0ba3e5f3d38f4e922770811544e64` |
| Built from commit | Selfhost compiler source at `4e746666` plus the host-linker/Rust retirement changes in this worktree |
| Build target | `wasm32` / `wasi-p1` direct core (guest `(memory 8192)` **memory32**) |
| Producer | Direct Wasmtime selfhost compile using the validated Memory64 bootstrap candidate; no host-linker, bridge, adapter, or Rust code |

## Reproducibility recipe

The pinned wasm is a direct core compiler artifact. Component packaging is a
separate official-tooling step: `wasm-tools component embed/new` consumes the
standard WASI P2 core and generated WIT. A minimal verification recipe is:

```bash
# 1. Verify the direct bootstrap
wasm-tools validate bootstrap/arukellt-selfhost.wasm
scripts/run/arukellt-selfhost.sh --version

# 2. Verify the official P2 component path
scripts/run/arukellt-selfhost.sh compile program.ark \
  --target wasm32-gc --wasi-version wasi-p2 --emit component \
  -o .build/program.component.wasm
wasm-tools validate .build/program.component.wasm
```

The `selfhost fixpoint` gate remains the stricter source-refresh check when a
current stage-2 build is available; it is not required to execute the direct
pinned bootstrap or the official component packaging path above.

## Refresh policy

The pinned wasm is **explicitly refreshed**, never auto-bumped. Refresh is
required when an intentional behavioural change in the selfhost compiler
(`src/compiler/**`) makes the four gates fail against the previous pinned
reference. Refresh procedure:

1. Locally bootstrap a new direct compiler wasm from the previous pinned
   reference and the new compiler source.
2. When the stage-2 build is available, verify the Stage-3 fixpoint (`s2 ==
   s3`). If the refresh path needs an intermediate Stage-3 artifact, verify one
   more round (`s3 == s4`) and pin the stable fixpoint artifact.
3. Run the full fixture-parity gate against the previous pinned reference and
   review every difference. Document each behavioural drift in the refresh
   commit message; if any drift is unintentional, **do not refresh**.
4. Replace `bootstrap/arukellt-selfhost.wasm` with the new fixpoint binary,
   update this file's *sha256*, *size*, and *Built from commit* rows, and
   commit both changes in one commit titled
   `chore(bootstrap): refresh pinned selfhost wasm to <short-sha>`.

The refresh commit must be signed off by a maintainer and mention every
behavioural drift in its body.

### Direct bootstrap and WASI P2 packaging

Pinned bootstrap is direct `wasm32` / `wasi-p1` with guest memory32
(`(memory 8192)`). The compiler emits standard `wasm32-gc` / `wasi-p2` core
modules for component work. `scripts/run/arukellt-selfhost.sh` packages those
modules with the official WASI P2 WIT and `wasm-tools component embed/new`.
There is no host-linker or compatibility adapter in either path.

## Why this artifact is committed

The four selfhost gates (`fixpoint`, `fixture-parity`, `diag-parity`,
and CLI parity) start from this binary so a fresh clone can verify the
selfhost compiler without a prior build. See ADR-029 for the contract.
