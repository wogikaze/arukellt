# Pinned-reference selfhost wasm — Provenance

This directory holds the **committed pinned-reference selfhost wasm** used as
the trusted bootstrap base for selfhost verification (see
[ADR-029](../docs/adr/ADR-029-selfhost-native-verification-contract.md)).

The pinned wasm is the source-of-truth for bootstrapping on a fresh clone.
It is a direct WASI Preview 1 core module executed by Wasmtime; no Rust binary,
Cargo workspace, host-linker, or repository-specific runtime is involved.

## Artifact

| Field | Value |
|-------|-------|
| Path | `bootstrap/arukellt-selfhost.wasm` |
| Size | 11 503 141 bytes (≈ 10.97 MiB) |
| sha256 | `a2803831acaa5fac8c1f40767789c5ed12b87c2a5eebdaeb2f0dca896874e156` |
| Built from commit | Selfhost compiler source at `58cc153e0` (the #922 linear-GC chain: boundary-aware marking, class-segregated free lists, arena capacity, heap-start memory sizing) |
| Build target | `wasm32` / `wasi-p1` direct core (guest `(memory 8199)`, ≈ 512.7 MiB **memory32** — min pages now sized from the emitted heap start, `max(8192, ceil(heap_start/64KiB))`) |
| Producer | Fixpoint artifact of the ADR-029 stage-2 → stage-3 chain run in an isolated worktree from the previous pinned artifact (`3f0ca605…`); the artifact recompiles the flat-src overlay **byte-identically** (`scale_v28.wasm == lin_comp_v28.wasm`, sha256 `a2803831…`), making it the stable fixpoint. No host-linker, bridge, adapter, or Rust code |

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

Normal fixture testing is intentionally **not** pinned-vs-current parity.
`python3 scripts/manager.py selfhost fixture-parity` now builds the current
selfhost compiler once, compiles and validates every `run:` fixture in parallel,
and executes only a bounded cross-domain runtime smoke set (plus explicit trap
contracts). This keeps fixture verification on the normal development path
without paying for two compiler generations per fixture.

Pinned-vs-current fixture comparison is a bootstrap-refresh audit only:

```bash
python3 scripts/run/selfhost-fixture-test.py --reference
```

## Refresh policy

The pinned wasm is **explicitly refreshed**, never auto-bumped. Refresh is
required when an intentional behavioural change in the selfhost compiler
(`src/compiler/**`) makes the bootstrap verification contract fail against the
previous pinned reference. Refresh procedure:

1. Locally bootstrap a new direct compiler wasm from the previous pinned
   reference and the new compiler source.
2. When the stage-2 build is available, verify the Stage-3 fixpoint (`s2 ==
   s3`). If the refresh path needs an intermediate Stage-3 artifact, verify one
   more round (`s3 == s4`) and pin the stable fixpoint artifact.
3. Run `python3 scripts/run/selfhost-fixture-test.py --reference` against the
   previous pinned reference and review every difference. This is deliberately
   outside normal fixture testing. Document each behavioural drift in the
   refresh commit message; if any drift is unintentional, **do not refresh**.
4. Replace `bootstrap/arukellt-selfhost.wasm` with the new fixpoint binary,
   update this file's *sha256*, *size*, and *Built from commit* rows, and
   commit both changes in one commit titled
   `chore(bootstrap): refresh pinned selfhost wasm to <short-sha>`.

The refresh commit must be signed off by a maintainer and mention every
behavioural drift in its body.

### Direct bootstrap and WASI P2 packaging

Pinned bootstrap is direct `wasm32` / `wasi-p1` with guest memory32
(`(memory 8199)`, ≈ 512.7 MiB). The compiler emits standard `wasm32-gc` /
`wasi-p2` core modules for component work. `scripts/run/arukellt-selfhost.sh`
and the fixture gate package those modules with the official WASI P2 WIT and
`wasm-tools component embed/new`. There is no host-linker or compatibility
adapter in either path.

## Why this artifact is committed

Fixpoint, diagnostic parity, CLI parity, and construction of the current
selfhost compiler used by the fixture gate all start from this binary. A fresh
clone can therefore verify the compiler without a prior local build. Pinned
fixture-output comparison is reserved for explicit artifact refreshes instead
of being multiplied across every normal fixture test.
