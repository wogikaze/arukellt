# Pinned-reference selfhost wasm — Provenance

This directory holds the **committed pinned-reference selfhost wasm** used as
the trusted bootstrap base for selfhost verification (see
[ADR-029](../docs/adr/ADR-029-selfhost-native-verification-contract.md)).

## Official artifact (wasm32-gc / wasi-p2 host component)

`bootstrap/arukellt-selfhost.wasm` is the current stage-2 compiler packaged as
a `wasm32-gc` / `wasi-p2` command component, so the compiler itself executes
on Wasmtime's GC instead of the linear-memory host. Compiling the bootstrap
overlay with this pin takes ~40 s (the previous core pin took ~34 min on the
same source) and RSS is roughly half.

| Field | Value |
|-------|-------|
| Path | `bootstrap/arukellt-selfhost.wasm` |
| Size | 8 513 357 bytes |
| sha256 | `3c02d18b254709e25065a0e06c82bf08f8fd54a6ad7378a85a61fb5e433a4ee9` |
| Built from commit | `5784e58af` (selfhost source baked into this artifact) |
| Build target | `wasm32-gc` / `wasi-p2` component, cm32p2 command wrapper |
| Producer | A gc compiler built from the same source by the previous core pin compiled the overlay to `wasm32-gc` with `--emit component`; `wasm-tools component embed/new` packaged it |
| p1 identity | Compiling the overlay with `--target wasm32 --wasi-version wasi-p1` writes `56e7ff6af053b8add4ee310213092a581dbdda5661b1d7850c73beddde33eebd`, byte-identical to the stage-2 s2 the previous core pin produced |
| Self-reproduction | Compiling the overlay to `wasm32-gc --emit component` writes core `93aa0e6bf39779e5803b0ba650666c969ede0fb8e60216025e281792726420ac`, the core this component packages |

Why the change: the 2026-10-28 core pin refresh retired the gc-host component
because the current compiler did not emit a valid `wasm32-gc` self-build.
The cause was a source bug, not a gc-host limit: `lsp::documents` called
`JsonText::get_str_after`, which did not exist on the `JsonText` impl, so the
lowering fell back to a trap stub whose result poisoned the enclosing
if-expression join. With the associated function added (`std/json.ark`), the
gc self-build validates and the gc host is restored as the official artifact.

`BOOTSTRAP_EMIT_*` remains `wasm32` / `wasi-p1`: the pin is a **host
accelerator**, not a change of the stage-2 output target. Flipping the emitted
target stays gated on ADR-053's <=10 s overlay acceptance.

The flat overlay strips `mut` parameter modes for pre-#951 pins. A gc pin
built from the current source therefore runs the overlay with
`--bootstrap-allow-shared-param-mutation`; the Python compile path passes that
flag and reuses the shared AOT image cache for the component.

## Previous core pin (recoverable)

The 2026-10-28 official core pin was 11 585 962 bytes, sha256
`8df2698c8401693879ca4f9ced289ed2c285d95eb526cf70fda02dd5876259ee`
(commit `fb5944edb`). It is no longer the official slot but remains the
recovery base for a core-only environment:

```bash
git show fb5944edb:bootstrap/arukellt-selfhost.wasm > bootstrap/arukellt-selfhost.wasm
```

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

Verify the component pin and compile a program with it:

```bash
# 1. Validate the component and check the CLI runs on Wasm GC.
wasm-tools validate --features component-model bootstrap/arukellt-selfhost.wasm
scripts/run/arukellt-selfhost.sh --version

# 2. Compile a program with the pin.
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
previous pinned reference. Refresh procedure for the gc component pin:

1. Build the current compiler and a component-ready gc self-build from it.
   `python3 scripts/manager.py selfhost build-compiler` writes the stage-2 s2;
   compile the flat overlay with an existing gc pin (or that s2) using
   `--target wasm32-gc --wasi-version wasi-p2 --emit component`. The
   `--emit component` pass is required: packaging a plain gc core without it
   fails the canonical `realloc` signature check.
2. Package with the official WASI P2 WIT:
   `wasm-tools component embed scripts/selfhost/wit/deps/wasi-cli-0.2.0/wit
   --world command <core> -o embedded.wasm` and
   `wasm-tools component new embedded.wasm --reject-legacy-names
   --realloc-via-memory-grow -o <component>`, then validate with
   `wasm-tools validate --features component-model`.
3. Verify the accelerator contract: the component compiling the overlay to
   `--target wasm32 --wasi-version wasi-p1` must write the same stage-2 s2 as
   the previous pin produced, and compiling the overlay to
   `wasm32-gc --emit component` must reproduce the pin's own core sha256.
4. Run `selfhost build-compiler`, `selfhost fixpoint`, and
   `python3 scripts/run/selfhost-fixture-test.py --reference` against the
   previous pinned reference; review every difference. If any drift is
   unintentional, **do not refresh**.
5. Replace `bootstrap/arukellt-selfhost.wasm` with the new component, update
   this file's *sha256*, *size*, *Built from commit*, *p1 identity*, and
   *Self-reproduction* rows, and commit both changes in one commit titled
   `chore(bootstrap): refresh pinned selfhost wasm to <short-sha>`.

The refresh commit must be signed off by a maintainer and mention every
behavioural drift in its body.

### Component pin and component output

The official pin is a `wasm32-gc` / `wasi-p2` command component. The compiler
still emits `wasm32-gc` / `wasi-p2` core modules for component work.
`scripts/run/arukellt-selfhost.sh` and the fixture gate package those modules
with the official WASI P2 WIT and `wasm-tools component embed/new`. There is
no host-linker or compatibility adapter in either path.

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
