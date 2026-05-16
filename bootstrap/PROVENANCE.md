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
| Size | 862 369 bytes (≈ 842 KiB) |
| sha256 | `f62ecf3b863338916998c65c58c9fd2c8ad42d37e7fe0cfc125eb989341e0f8a` |
| Built from commit | `7911a527` plus the current #312/#121/#034 selfhost worktree changes, including the #121 Result<i32,String>, Result<bool,bool>, tuple<i32,bool>, tuple<bool,bool> parameter, tuple<i64,i64> result, f32 binary/f32-param-i32/f32-result-i32 adapters, option<bool>/option<i64> result/parameter adapters, i64 payload variant lowering, and record-add alias adapter fixes |
| Build target | `wasm32-wasi-p1` |
| Producer | Selfhost compiler stage 2 (`s2.wasm`), confirmed by Stage-3 fixpoint |

## Reproducibility recipe

The pinned wasm is the deterministic Stage-2 output of the selfhost compiler
when compiled from the recorded source commit plus the refresh worktree:

```bash
# 1. Check out the recorded source commit
git checkout 7911a527

# 2. Rebuild Stage-1 selfhost wasm using the previous pinned reference
mkdir -p .build/selfhost
wasmtime run --dir=. bootstrap/arukellt-selfhost.wasm -- \
  compile src/compiler/main.ark \
  --target wasm32-wasi-p1 \
  -o .build/selfhost/arukellt-s2.wasm

# 3. Verify byte-for-byte identity with the pinned wasm
sha256sum bootstrap/arukellt-selfhost.wasm .build/selfhost/arukellt-s2.wasm
# ⇒ both sums must be identical.
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
2. Verify the Stage-3 fixpoint holds (`s2 == s3`). If the refresh path needs an
   intermediate Stage-3 artifact, verify one more round (`s3 == s4`) and pin the
   stable fixpoint artifact.
3. Run the full fixture-parity gate against the previous pinned reference and
   review every difference. Document each behavioural drift in the refresh
   commit message; if any drift is unintentional, **do not refresh**.
4. Replace `bootstrap/arukellt-selfhost.wasm` with the new fixpoint binary,
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

The artifact size (~842 KiB) is well under the 10 MiB ceiling discussed in
ADR-029. This file is exempted from the repo-wide `*.wasm` ignore in
`.gitignore` via an explicit allow-list entry.
