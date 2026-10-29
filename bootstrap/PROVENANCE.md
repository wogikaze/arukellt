# Pinned-reference selfhost wasm — Provenance

This directory holds the **committed pinned-reference selfhost wasm** used as
the trusted bootstrap base for selfhost verification (see
[ADR-029](../docs/adr/ADR-029-selfhost-native-verification-contract.md)).

## Official artifact (wasm32-gc / wasi-p2 host component)

`bootstrap/arukellt-selfhost.wasm` is the current stage-2 compiler packaged as
a `wasm32-gc` / `wasi-p2` command component, so the compiler itself executes
on Wasmtime's GC instead of the linear-memory host. Compiling the bootstrap
overlay with this pin takes ~30–34 s warm (the previous core pin took ~34 min
on the same source; the first run after a pin refresh adds the component's AOT
image build).

| Field | Value |
|-------|-------|
| Path | `bootstrap/arukellt-selfhost.wasm` |
| Size | 8 540 660 bytes |
| sha256 | `56ff0808171dcc3c521b0e85d07f2b16a349a59419483ad71148a5ee928d23e8` |
| Built from commit | `ff4ff6a4f` (clone-retirement merge regression and bootstrap stub restored; `ba4aa88b1` + `ff4ff6a4f`) |
| Build target | `wasm32-gc` / `wasi-p2` component, cm32p2 command wrapper |
| Producer | Generation-2 s2 `61aa349b…` (built by generation-1 s2 `e8a0bdb1…`, itself built by the committed pin `471855ad…`) compiled the overlay to `wasm32-gc` with `--emit component`; `wasm-tools component embed/new` packaged it |
| p1 identity | Compiling the overlay with `--target wasm32 --wasi-version wasi-p1` writes `61aa349b377432773354e633182648becd8da9c97343e53bbe57cf192b73c62d`, byte-identical to the generation-2 s2 this component was built from (fixed point) |
| Self-reproduction | Compiling the overlay to `wasm32-gc --emit component` writes core `0ebeb365d2833ae6850ff0482b62a20fbd4298de8001f3d94cc812a817a2fd98`, the core this component packages |

The 2026-10-30 refresh is the first one that needed two s2 generations
(generation convergence). The committed pin (`471855ad…`, built from
`36f42b97e`) produced stage-2 `e8a0bdb1…` (11 816 406 B); that s2 produced
`61aa349b…` (11 817 385 B), which is a fixed point: the packaged component's
p1 output is `61aa349b…` and its gc self-build is `0ebeb365…` (8 532 895 B;
component `56ff0808…`, 8 540 660 B). The +842-byte p1 drift is the accumulated
emission change of the compiler commits between the pin source and this tree,
not a host difference.

The generation hop also exposed a merge regression: `96a66db83` had kept
`93582a23e`'s pre-clone versions of eight MIR lowerer files, and the hard-coded
`mir_lower_ctx_mono_type_params.ark` bootstrap stub still emitted bare
`clone(x)`. Against the merged prelude's generic `clone<T>` shim the gc
self-build resolved those calls to unrelated `Vec__clone__…` instances and
emitted invalid core Wasm (func 7686: `type mismatch: expected (ref null
$type), found (ref null $type)`). `ba4aa88b1` restores the clone migration in
the lowerers and `ff4ff6a4f` migrates the stub; with both, the gc self-build
validates. The pin was not installed until the fixed source converged.

The `3c02d18b` refresh restored the gc host after the 2026-10-28 core pin
retired it. The `35a2585a` refresh baked in the 2026-10-29 allocation-free
matcher changes (`corehir::name_match`, enum-variant / callee spelling /
signature-registry leaf comparisons). The `783be1b8f` refresh baked in the
#944 E0212 non-call move diagnostics; this `36f42b97e` refresh bakes in the
#965 Vec<Ord> projection, the HashSet retirement, and the resolver fixes that
made that series build. Each refresh re-verified p1 identity and
self-reproduction against the previous pin; the host swap itself does not
change the emitted s2. The s2 for that `36f42b97e` source was `353473ae…`; the
current fixed-point s2 is `61aa349b…` for the `ff4ff6a4f` source.
HEAD advanced while this refresh was prepared (resolver fixes + merged
HashSet hint restoration); the new pin builds that newer source
(s2 `1eb4a81c…`, fixpoint PASS, fixture parity PASS).

The `3c02d18b` refresh restored the gc host after the 2026-10-28 core pin
retired it. The `35a2585a` refresh baked in the 2026-10-29 allocation-free
matcher changes (`corehir::name_match`, enum-variant / callee spelling /
signature-registry leaf comparisons). This `783be1b8f` refresh bakes in the
#944 E0212 non-call move diagnostics. Each refresh re-verified p1 identity and
self-reproduction against the previous pin; the host swap itself does not
change the emitted s2. The current s2 is `d917f7cd…` for the #944 source, and
the previous pin produces the same bytes from that source.

Why the gc host was restored (2026-10-28): the core pin refresh retired the
gc-host component because the then-current compiler did not emit a valid
`wasm32-gc` self-build. The cause was a source bug, not a gc-host limit:
`lsp::documents` called `JsonText::get_str_after`, which did not exist on the
`JsonText` impl, so the lowering fell back to a trap stub whose result
poisoned the enclosing if-expression join. With the associated function added
(`std/json.ark`), the gc self-build validates and the gc host is the official
artifact.

`BOOTSTRAP_EMIT_*` remains `wasm32` / `wasi-p1`: the pin is a **host
accelerator**, not a change of the stage-2 output target. Flipping the emitted
target stays gated on ADR-053's <=10 s overlay acceptance.

The flat overlay strips `mut` parameter modes so the pre-#951 core
trust-anchor/override pins can compile it. The shared overlay is therefore
mode-free, and both self-compile stages pass
`--bootstrap-allow-shared-param-mutation`. The official gc pin includes #951
and would accept the real modes, but the strip keeps the stage-2 source and
emitted s2 identical across pins. The Python compile path passes the flag and
reuses the shared AOT image cache for the component.

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
3. Verify the accelerator contract at a fixed point: the component compiling the
   overlay to `--target wasm32 --wasi-version wasi-p1` must write the same
   stage-2 s2 it was generated from, and compiling the overlay to
   `wasm32-gc --emit component` must reproduce the pin's own core sha256. When
   the pin's source generation is behind the tree, one s2 hop is not enough:
   compute `s2_1 = pin(source)`, `s2_2 = s2_1(source)`, and repeat until the
   s2 stops changing; package the gc self-build of the fixed-point s2. A
   one-step host swap is the special case where `s2_1` is already the fixed
   point.
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
