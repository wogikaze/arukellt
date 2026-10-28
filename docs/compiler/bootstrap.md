# Selfhost Bootstrap (ADR-029)

> **Current contract.** Trusted base is the pinned selfhost wasm. Rust-era
> Stage 0 narratives live in
> [`../history/reports/bootstrap-rust-era-compiler-guide.md`](../history/reports/bootstrap-rust-era-compiler-guide.md).

Normative decision: [`../adr/ADR-029-selfhost-native-verification-contract.md`](../adr/ADR-029-selfhost-native-verification-contract.md).  
Status summary: [`../state/compiler.md`](../state/compiler.md).  
Structured SSOT: [`../data/bootstrap-contract.toml`](../data/bootstrap-contract.toml).

The official pinned bootstrap is a direct `wasm32` / `wasi-p1` core module
(`bootstrap/arukellt-selfhost.wasm`, memory32). It emits the stage-2/stage-3
compiler as `wasm32` / `wasi-p1` core modules with memory32; the emitted
compiler contract is unchanged. The previous core pin stays at
`bootstrap/arukellt-selfhost-core.wasm` as the trust anchor and as an explicit
override. User-facing `wasm32-gc` / `wasi-p2` output is still packaged as an
official WASI component with the checked-in WIT world.

The core override pin predates generic enum payload lowering used by the
current `std/prelude.ark`. `selfhost build-compiler` keeps the structural
`Box` / `Slice` retry for that path. The checked-in prelude, user source
semantics, and emitted user artifacts are not rewritten.

The pin's builtin host modules only expose the free `std::host::*` functions,
so the flat overlay ships the namespace sources the compiler itself imports:
`std/host/fs.ark`, `std/host/clock.ark`, and `std/io/mod.ark`. The checked-in
`Fs::*`, `Clock::*`, and `toml::TomlText::*` spellings therefore compile to the
real impls instead of the unresolved-call `drop; unreachable` fallback. The
overlay no longer rewrites those call spellings or installs
`bootstrap_toml_*` adapters.

One host namespace remains a pin limitation: `8df2698c…` predates #917's MIR
env call-result typing, so a reachable `Env::args` (a `Vec<String>` result)
still produces invalid core Wasm. `_patch_bootstrap_host_env_paths` keeps the
compiler's `Env::*` calls on the free builtin spelling, and the entry module
uses the prelude `args` / `exit` / `println` host-intrinsic stubs. Delete both
when the pin includes the #917 typing; the function docstrings carry the probe
that must validate the lowering first.

The isolated WIT parser closure in that overlay also normalizes qualified type
names (`types::WitType`, `ast::WitNode`, and related record types) to the flat
overlay namespace. Function calls remain qualified, and the checked-in `std/`
sources are not rewritten. This is a bootstrap namespace adapter only; it does
not relax the language type identity or make an s2 build equivalent to an
ADR-029 fixpoint.

## Which command?

Agents and humans confuse these two. Use the table:

| Goal | Command | Typical time | Notes |
|------|---------|--------------|-------|
| **Refresh the compiler after editing `src/compiler/**`** | `python3 scripts/manager.py selfhost build-compiler` | ~25-30 min with the official core pin; ~40-60 s with `--fast` | Stage-2 only. **Default for emitter / Memory64 / T3 work.** |
| Check ADR-029 fixpoint (`sha256(s2) == sha256(s3)`) | `python3 scripts/manager.py selfhost fixpoint` | seconds if s2/s3 exist | Does not refresh the emitter by itself |
| Rebuild s2 **and** s3 then compare (gate only) | `python3 scripts/manager.py selfhost fixpoint --build` | stage-3 alone can be **tens of minutes** today on gc+p2 | **Not** for routine iteration. gc-host ≤10s is [#851](../../issues/open/851-selfhost-compiler-core-rewrite.md) / [ADR-053](../adr/ADR-053-selfhost-compiler-core-rewrite.md) |

Aliases for `build-compiler`: `build-s2`, `rebuild-s2`.

Do **not** use `fixpoint --build --no-cache` to “just rebuild s2” — that also
runs stage-3 and floods long builds under parallel agents.

Copy files with `/bin/cp -f` (never interactive `cp -iv`).

### Bootstrap pin

`bootstrap/arukellt-selfhost.wasm` is the official direct `wasm32` /
`wasi-p1` core pin (memory32, 8192 pages; sha256 `8df2698c…`).
`selfhost build-compiler` compiles the flat overlay through it to the stage-2
compiler:

```bash
python3 scripts/manager.py selfhost build-compiler
```

A full self-build takes ~25-30 minutes because the retired gc host is not a
faster path. The previous core pin is at
`bootstrap/arukellt-selfhost-core.wasm` and can be selected with
`ARUKELLT_PINNED_WASM=bootstrap/arukellt-selfhost-core.wasm`; that path costs
20-40 minutes. The ADR-029 fixpoint is still required before a tag. Provenance
and removal conditions:
[`bootstrap/PROVENANCE.md`](../../bootstrap/PROVENANCE.md).

### How to iterate without unnecessary rebuilds

`build-compiler` is a **full compile of the entire selfhost compiler**
(typecheck + MIR lower + wasm emit). Overlay cache hits only skip the flat-src
rewrite; they do **not** skip that compile.

**Do not rebuild once per one-line hypothesis.** That makes agents
latency-bound (`8s × N` tries).

Recommended loop:

1. Classify failures / read WAT with the **current** s2 (no rebuild).
2. Batch all planned `src/compiler/**` edits.
3. **One** `selfhost build-compiler --fast` (~60 s with the iteration pin).
4. Re-validate **many** fixtures against that s2. Use
   `selfhost fixture-parity --filter-dir <area>` while a batch is in flight,
   and `ARUKELLT_FIXTURE_WORKERS=12 verify fixtures` at a checkpoint.
5. Only rebuild again after the next batch of source edits.

Parallel agents must **share** one rebuilt s2 (parent rebuilds once); each lane
must not run its own `build-compiler`.

### wasm32 overlay compile: preopen the overlay only

The flat overlay copies a trimmed `std/prelude.ark` (it strips
`use std::collections::*`). Measuring or emitting a wasm32 compiler with
`--dir=<repo>` as well lets `std/prelude.ark` resolve to the full tree, which
pulls `impl String` methods. On wasm32 those methods rewrite `len`/`slice` and
the successor leaks (multi-GiB) during lower.

`scripts/selfhost/checks.py` `_wasm_compile` therefore preopens **only** the
overlay workspace when one is set. Manual timing must do the same:

```bash
wasmtime run --allow-precompiled --wasm gc --wasm function-references \
  -W memory64=y -W max-memory-size=17179869184 \
  --dir=.build/selfhost/flat-src \
  HOST.cwasm -- \
  compile src/compiler/main.ark --target wasm32 --wasi-version wasi-p1 \
  -o out.wasm
```

The bootstrap host emits a direct core module for the configured bootstrap
profile. A command component is packaged from standard P2 core imports with
the official `wasm-tools component embed` / `component new` commands; no
adapter or bridge is permitted.

## Trust model

| Stage | Artifact / check | Command |
|-------|------------------|---------|
| **0 (trust base)** | `bootstrap/arukellt-selfhost.wasm` (pinned; see `bootstrap/PROVENANCE.md`) | — |
| **Build current selfhost** | pinned compiles `src/compiler/main.ark` → `.build/selfhost/arukellt-s2.wasm` | `python3 scripts/manager.py selfhost build-compiler` |
| **Fixpoint** | `sha256(s2) == sha256(s3)` | `python3 scripts/manager.py selfhost fixpoint` |
| **Verification** | current-only fixture test / CLI / diag | `python3 scripts/manager.py selfhost fixture-parity`, `… parity --mode --cli`, `… diag-parity` |

Stage-3 / runtime compiler wasms are validated with `wasm-tools validate` after
build (and when reused). An invalid artifact — for example Memory64 GC output
that does `struct.set` of an i32 field without `i32.wrap_i64` — is deleted and
must not remain as a selectable `arukellt-s3.wasm`. The day-to-day wrapper
prefers `.build/selfhost/arukellt-s2-runtime.wasm` over s3 for this reason.

Stage 0 is **the pinned wasm**. The selfhost launcher has no Rust compiler
fallback. `ARUKELLT_USE_RUST=1` is rejected by the launcher.

## User-facing entrypoint

```bash
# Preferred: wrapper resolves pinned / s2 / env override
scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark --target wasm32-gc

# Or point ARUKELLT_SELFHOST_WASM at a freshly built s2 for library component work
ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
  scripts/run/arukellt-selfhost.sh compile lib.ark --target wasm32-gc --emit component
```

Resolution order (wrapper `scripts/run/arukellt-selfhost.sh`, with each
existing candidate rejected if it uses a retired ABI):
`$ARUKELLT_SELFHOST_WASM` → `bootstrap/arukellt-selfhost.wasm` →
`.build/selfhost/arukellt-s2-runtime.wasm` → `.build/selfhost/arukellt-s3.wasm` →
`.build/selfhost/arukellt-s2.wasm` → `.bootstrap-build/arukellt-s2.wasm` →
`.build/selfhost/arukellt-pinned-bootstrap.wasm`.

## Artifact policy

| Path | Status |
|------|--------|
| Legacy P1-shaped P2 core imports | Rejected; no compatibility adapter |
| Repository-specific host imports | Rejected; use official WASI interfaces |
| Rust compiler fallback | Removed from the product and verification paths |

Historical walkthroughs of the old Rust→s1→s2 script remain under `docs/history/reports/`.

## CI

Bootstrap evidence runs in the **`selfhost`** job of `.github/workflows/ci.yml`
(fixpoint + parity). Do not invent a `verification-bootstrap` job name.
