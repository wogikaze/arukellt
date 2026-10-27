# Pinned-reference selfhost wasm — Provenance

This directory holds the **committed pinned-reference selfhost wasm** used as
the trusted bootstrap base for selfhost verification (see
[ADR-029](../docs/adr/ADR-029-selfhost-native-verification-contract.md)).

## Official artifact (wasm32 / wasi-p1 core)

`bootstrap/arukellt-selfhost.wasm` is the current stage-2 compiler core built
from the checked-in selfhost source at `be7f6d3d9` through the bootstrap
overlay. It is a direct `wasm32` / `wasi-p1` core module with memory32 (8192
pages) and reproduces itself byte-for-byte: compiling the same bootstrap
overlay with this binary writes the same bytes back out.

| Field | Value |
|-------|-------|
| Path | `bootstrap/arukellt-selfhost.wasm` |
| Size | 11 585 962 bytes |
| sha256 | `8df2698c8401693879ca4f9ced289ed2c285d95eb526cf70fda02dd5876259ee` |
| Built from commit | `be7f6d3d9` (selfhost source baked into this artifact) |
| Build target | `wasm32` / `wasi-p1` direct core, memory32 (8192 pages) |
| Producer | The previous stage-2 (`cf179acd…`, built by the 2026-10-26 gc pin from the patched overlay) compiled the checked-in source with the `Fs::*` / `TomlText::*` spellings (27 min) |
| Self-reproduction | `8df2698c…` compiles the same overlay to `8df2698c…` byte-for-byte |

Why the change: the 2026-10-26 gc-host component pin predates the non-main
associated/namespace call lowering used by the current compiler source. The
generated overlay routed the compiler's `TomlText::*` and `Fs::*` calls through
free shims so the old pin could compile it. The current stage-2 resolves those
spellings when the overlay ships their namespace sources (`std/host/fs.ark`,
`std/io/mod.ark`), so the pin was refreshed to the current stage-2 and the two
adapter patches were deleted.

The gc-host component pin is retired from the official slot: the current
compiler does **not** emit a valid `wasm32-gc` self-build (the gc self-compile
fails validation), so a gc component cannot be refreshed from this source yet.
The pin is the `wasm32` / `wasi-p1` core that `selfhost build-compiler`,
`selfhost fixpoint`, and the fixture gate actually execute. Restoring a
gc-host pin requires fixing the gc self-build first; do not fake it by
packaging an invalid core module.

## Trust anchor (previous core pin)

`bootstrap/arukellt-selfhost-core.wasm` is the 2026-10-19 core pin. It stays
committed as an explicit override (`ARUKELLT_PINNED_WASM=bootstrap/arukellt-selfhost-core.wasm`).

| Field | Value |
|-------|-------|
| Path | `bootstrap/arukellt-selfhost-core.wasm` |
| Size | 11 503 141 bytes (≈ 10.97 MiB) |
| sha256 | `a2803831acaa5fac8c1f40767789c5ed12b87c2a5eebdaeb2f0dca896874e156` |
| Built from commit | Selfhost compiler source at `58cc153e0` (#922 linear-GC chain) |
| Build target | `wasm32` / `wasi-p1` direct core, memory32 (≈ 512.7 MiB) |
| Producer | Fixpoint artifact of the ADR-029 stage-2 → stage-3 chain from the previous pin (`3f0ca605…`); recompiles the flat overlay byte-identically (`a2803831…`) |

## Reproducibility recipe

The pinned wasm is a direct core compiler artifact and needs no component
packaging. A minimal verification recipe is:

```bash
# 1. Verify the direct bootstrap
wasm-tools validate bootstrap/arukellt-selfhost.wasm
scripts/run/arukellt-selfhost.sh --version

# 2. Compile a program with the pin
scripts/run/arukellt-selfhost.sh compile program.ark --target wasm32 -o out.wasm
wasm-tools validate out.wasm
```

Normal fixture testing is intentionally **not** pinned-vs-current parity.
`python3 scripts/manager.py selfhost fixture-parity` builds the current
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

1. Locally bootstrap a new direct compiler wasm from the previous stage-2 and
   the new compiler source (the canonical `selfhost build-compiler` path, or a
   manual `compile src/compiler/main.ark --target wasm32 --wasi-version
   wasi-p1` against the overlay).
2. Verify the new pin reproduces itself: compiling the same overlay with it
   must write the same sha256 back. This is the stage-2/stage-3 fixpoint
   (`s2 == s3`) for a direct core pin.
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

### Direct core pin and component output

Pinned bootstrap is direct `wasm32` / `wasi-p1` with guest memory32
(`(memory 8192)`, 8192 pages). The compiler still emits standard
`wasm32-gc` / `wasi-p2` core modules for component work.
`scripts/run/arukellt-selfhost.sh` and the fixture gate package those modules
with the official WASI P2 WIT and `wasm-tools component embed/new`. There is
no host-linker or compatibility adapter in either path.

The gc-host component pin introduced on 2026-10-26 is no longer the official
artifact. It cannot be refreshed until the current compiler emits a valid
`wasm32-gc` self-build; see the refresh commit for the failing validation
evidence.

## Why this artifact is committed

Fixpoint, diagnostic parity, CLI parity, and construction of the current
selfhost compiler used by the fixture gate all start from this binary. A fresh
clone can therefore verify the compiler without a prior local build. Pinned
fixture-output comparison is reserved for explicit artifact refreshes instead
of being multiplied across every normal fixture test.

## Legacy iteration pin (fallback only)

`bootstrap/arukellt-selfhost-fast.wasm` is the previous 10/10 core pin kept for
`wasm32` / `wasi-p1` smoke work. Its s2 is not fixture-valid on the `wasm32-gc`
path (`stdlib_trait/iterator_string_callbacks.ark` hangs with
`--memory-width memory32`), so do not use it for the fixture or fixpoint gates.
Remove it when no lane needs the old p1 bootstrap.
