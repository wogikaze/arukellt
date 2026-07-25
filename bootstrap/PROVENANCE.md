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
| Size | 2 331 913 bytes (≈ 2.22 MiB) |
| sha256 | `48ad40ee4edde5193819b3b2cfd4a530b0740965cb78c616e5ac51fe9d02afd8` |
| Built from commit | `9951fd2b` — two-round pinned refresh to stable wasm32 fixpoint (prior `08dfbfcbf913…` → `06b61c60ddbb…` → `48ad40ee4edd…`); `sha256(pinned)==sha256(s2)==sha256(s3)`; BOOTSTRAP_EMIT remains wasm32/wasi-p1 until wasm32-gc self-emit validates (#730) |
| Build target | `wasm32` |
| Producer | Modular selfhost compiler Stage-2/3 fixpoint artifact (`arukellt-s2.wasm` / `arukellt-s3.wasm`). Emit remains `wasm32` until a validating `wasm32-gc` self-compile path exists (#730 follow-up: current s2→`wasm32-gc` emit fails `wasm-tools validate` at `func 8204`) |

## Reproducibility recipe

The pinned wasm is the deterministic Stage-2 output of the selfhost compiler
when compiled from the recorded source commit. Gates use
`scripts/selfhost/checks.py` which Memory64-widens the pinned wasm before
running it; a minimal recipe matching that emit target:

```bash
# 1. Check out the recorded source commit
git checkout 9951fd2b

# 2. Rebuild via the official gate (Memory64 bootstrap + overlay)
python3 scripts/manager.py selfhost fixpoint --build

# 3. Verify byte-for-byte identity with the pinned wasm
sha256sum bootstrap/arukellt-selfhost.wasm .build/selfhost/arukellt-s2.wasm
# ⇒ both sums must be identical.
sha256sum .build/selfhost/arukellt-s2.wasm .build/selfhost/arukellt-s3.wasm
# ⇒ identical sums = fixpoint reached
```

The `selfhost fixpoint` gate (`scripts/selfhost/checks.py::run_fixpoint`)
performs the bootstrap → s2 → s3 chain automatically.

## Refresh policy

The pinned wasm is **explicitly refreshed**, never auto-bumped. Refresh is
required when an intentional behavioural change in the selfhost compiler
(`src/compiler/**`) makes the four gates fail against the previous pinned
reference. Refresh procedure:

1. Locally bootstrap a new Stage-2 wasm from the previous pinned reference and
   the new compiler source (`python3 scripts/manager.py selfhost fixpoint --build`).
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

### wasm32-gc pinned (blocked)

Refreshing the pinned artifact to native `wasm32-gc` / `wasi-p2` / Memory64
emit is tracked in **#834** (split from #730). The former `func 8204`
`doc_parse_manifest` String-cast failure is fixed in source (`clone(T)→T`,
`06ba2d35`); full selfhost wasm32-gc compile/pin still needs a host that can
grow past 4GiB without ~21GiB RSS OOM. Keep `BOOTSTRAP_EMIT_TARGET = "wasm32"`
in `scripts/selfhost/checks.py` until #834 lands.

## Why this artifact is committed

The four selfhost gates (`fixpoint`, `fixture-parity`, `diag-parity`,
`cli-parity`) historically required `target/debug/arukellt` (the legacy Rust
compiler) as a trusted base, blocking the Phase 5 retirement work
(#560–#564). The pinned wasm replaces the Rust binary as the trusted base
and is committed so that:

- Fresh clones can bootstrap and verify without any Rust toolchain
- CI never needs to rebuild the Rust crate before running selfhost gates
- The bootstrap base is byte-pinned and reproducible from a git SHA

The artifact size (~2.22 MiB) is well under the 10 MiB ceiling discussed in
ADR-029. This file is exempted from the repo-wide `*.wasm` ignore in
`.gitignore` via an explicit allow-list entry.
